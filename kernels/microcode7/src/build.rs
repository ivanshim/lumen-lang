// Tokens to the seven forms, in one pass over the tokens. Names resolve
// to addresses here: a layer per routine, functions and bare blocks
// holding names, arms and loop bodies holding none and making no frame.
// RPLumen is read with a symbolic stack of forms.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::math;
use crate::scan::{Shape, Token};
use crate::table::{Blocks, Table};
use crate::form::{Clause, Input, Traps, Form, Prim, Routine, Address, Callee, Plan};
use crate::data::{Reach, Value};

type Res<T> = Result<T, String>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Holds {
    /// A function: names assigned inside are its own; nothing outside is written.
    Every,
    /// A bare block: names first assigned inside are its own.
    Fresh,
    /// A branch arm or loop body: nothing; names belong to the program around.
    Nothing,
}

thread_local! {
    static PARENT_PAYLOAD: Rc<Vec<Value>> = Rc::new(Vec::new());
}

/// What reading a class body gathers. `arms` counts the arms of
/// conditionals open around the member being read: only one arm of a
/// conditional runs, so a member named within one may go unwritten, and
/// `uncertain` holds those names, whose places are glanced at when the
/// class is built rather than read outright.
struct ClassParts {
    lexical_members: Vec<String>,
    completed_class: Address,
    needs_class_cell: bool,
    methods: Vec<(String, Rc<Routine>)>,
    attributes: Vec<String>,
    held: Vec<Form>,
    /// Every name the body has bound so far, each standing where the
    /// body bound it first and the methods among the rest. This alone
    /// settles what the namespace of the finished class shows.
    ranking: Vec<String>,
    /// Each annotated member's key with the routine answering its annotation.
    annotated_names: Vec<(Form, Form)>,
    uncertain: Vec<String>,
    arms: usize,
    cannot: bool,
    /// The address the body's own `locals()`/`vars()` is kept in, made
    /// before the body's first statement runs where the body spells
    /// either: a plain dict seeded with every member the body has
    /// bound so far, kept under an address of its own so a later
    /// asking reads the very dictionary this one made, and every
    /// member the body binds after this point keeps it in step.
    book: Option<Address>,
    /// Every name a statement of the body has bound as a plain member
    /// (never a method or a nested class), once `book` exists: what
    /// becomes of the class's member by that name is settled by
    /// `book` alone from then on, so a `del` through `locals()` -- or
    /// of the name itself -- leaves the class with no such member,
    /// rather than the value its place still happens to hold.
    book_tracked: HashSet<String>,
}

#[derive(Clone, Default)]
struct ScopeWords {
    bound: Vec<String>,
    outermost: Vec<(String, String)>,
    borrowed: Vec<String>,
    /// Which of those `borrowed` names a class body nested in this
    /// scope declared, rather than the scope's own statements: naming
    /// a parameter this way is no conflict, since the class is a scope
    /// of its own in the reference, unlike here, where it takes no
    /// layer and its declarations land where the layer's own would.
    class_borrowed: Vec<String>,
}

/// How a name stood in its scope before a `global` or `nonlocal`
/// named it: read, written, or annotated. A declaration coming after
/// the fact is a fault, worded by what came first.
const MET_READ: u8 = 0;
const MET_WRITTEN: u8 = 1;
const MET_ANNOTATED: u8 = 2;

struct Layer {
    permits_async: bool,
    async_walk_seen: bool,
    gathering_kind: Option<&'static str>,
    expression_targets: Vec<String>,
    comprehension: bool,
    borrowed: Vec<String>,
    class_borrowed: Vec<String>,
    /// The names this scope reads from the scopes around it, each with
    /// where it stands seen from this scope's own frame.
    reaching: Vec<Address>,
    holds: Holds,
    idents: Vec<String>,
    formals: Vec<String>,
    formal_slots: Vec<usize>,
    rpn: bool,
    /// Names bound to a global (by `global`, the same name; by `static`, a hidden one).
    aliases: Vec<(String, String)>,
    /// The names this scope read, wrote or annotated, in the order met:
    /// a `global` or `nonlocal` naming one after the fact is a fault.
    encountered: Vec<(String, u8)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Signature {
    pub arity: usize,
    pub yields: bool,
}

/// A postfix branch under construction: the formals its routine had when
/// the branch opened and how many added since the arm being read has
/// consumed. The two arms see one stack, so the second reuses the
/// formals the first took before it takes any of its own.
struct Fork {
    layer: usize,
    from: usize,
    taken: usize,
}

pub struct Builder<'a> {
    surveyed: HashMap<usize, ScopeWords>,
    survey: bool,
    kind_mark: Option<usize>,
    syntax_try_nesting: usize,
    finally_nesting: usize,
    in_lazy_from: bool,
    /// The class being read and what it is built on: what `self` and
    /// `parent` mean inside a method.
    within: Option<(String, Option<String>)>,
    receiver: Option<String>,
    class_bindings: Vec<(usize, HashMap<String, Address>)>,
    /// The names a class body under way has itself declared `global`,
    /// kept apart from `class_bindings`'s members and from the layer
    /// around the class: the declaration reaches only the body's own
    /// statements, standing at this very depth, never a routine nested
    /// in it nor the layer once the body is behind. Innermost class
    /// last, as `class_bindings` keeps them.
    class_globals: Vec<(usize, Vec<String>)>,
    /// What a class body under way read, wrote or annotated, one list
    /// a body, kept apart from the layer around it as the reference
    /// keeps a class's own scope apart.
    class_met: Vec<Vec<(String, u8)>>,
    /// Whether the binding being written is an import's: an import says
    /// nothing of a name for the sake of a later `global`, as the
    /// reference does not count one.
    importing: bool,
    /// The class bodies under way, innermost last: the parts each has
    /// gathered of its class so far.
    under_way: Vec<ClassParts>,
    /// Which parameters of each program take a name's own cell instead
    /// of a copy, read from the tokens before anything is built.
    shared_args: HashMap<String, Vec<bool>>,
    arg_names: HashMap<String, Vec<String>>,
    /// Whether the reading stopped over a thing the language calls a
    /// fault of the run rather than a program it could not read.
    stopped_fatally: bool,
    /// Whether the statement under way was marked asynchronous.
    asynchronous: bool,
    /// Whether the routine under way is a coroutine, and whether the
    /// routine read next is to be one.
    in_coroutine: bool,
    coroutine_next: bool,
    /// The values a case's pattern works out ahead of the subject.
    pattern_kinds: Vec<Form>,
    /// What the reference warns of in how a statement is written, found
    /// while reading, with the row and column of each, for whoever
    /// compiled the text to say through the warnings module.
    warnings: Vec<(String, u32, usize)>,
    /// The line the routine now being read was written on, which a
    /// fault raised on the way into it names.
    declared_at: u32,
    /// The slots the routine being built fills from what it carried
    /// away with it, gathered while its names are read.
    carrying: Vec<usize>,
    /// The type parameters the declaration just read wrote between
    /// brackets, taken by the routine that declaration is making.
    pending_types: Vec<String>,
    /// Where each bag of members a class may take in begins, by name:
    /// the token just past the mark that opens its body. Its members are
    /// read again wherever a class takes them in.
    bags: HashMap<String, usize>,
    /// Remarks the reading itself raised, which belong ahead of anything
    /// the program prints because they were noticed before it ran.
    noted_when_read: Vec<Form>,
    table: &'a Table,
    forks: Vec<Fork>,
    tokens: &'a [Token],
    original_words: &'a [Token],
    pos: usize,
    layers: Vec<Layer>,
    /// Whether this text was handed over while the run was already
    /// going, as text given to the word that reads text is and as a
    /// file asked for part way through is. A whole program built this
    /// way is a piece of a run in progress, not a run of its own.
    read_in: bool,
    /// Show expression statements in text compiled in interactive mode.
    interactive: bool,
    /// How many layers stood open before a word of this text was read.
    /// A statement is at the top of what was handed over when no more
    /// than these are open, whatever stands around them elsewhere.
    outer_layers: usize,
    /// Which names a `static` at the top of text handed over has spoken
    /// for. Nothing is bound there that would show a name said twice,
    /// so they are gathered here and counted.
    spoken_for: Vec<String>,
    gensyms: usize,
    gather_names: Vec<(String, String)>,
    pub presumed: HashMap<String, Signature>,
    pub seen: HashMap<String, Signature>,
    strict: bool,
    /// Settings of hidden globals for `static` names, run where the function is defined.
    statics: Vec<Form>,
    /// The parameters of the method just read that name properties too.
    also_property: Vec<String>,
    /// How many lines stand ahead of the program's own text, and
    /// whether the language says where a complaint happened at all.
    before: u32,
    /// Whether the reading has passed the library and reached the
    /// program's own text, and the outermost names that text has
    /// bound since: those alone stand ahead of a builtin word spelled
    /// the same, since the library may spell a builtin as a routine.
    past_library: bool,
    named_in_program: Vec<String>,
    native_exports: HashSet<String>,
    /// The file this text came out of, where it was read as the run
    /// went, so that every program built from it carries it.
    written_in: Option<Rc<str>>,
    /// The outermost dictionary the text under way was handed, where it
    /// came of a reading given one, and the builtins in force there:
    /// both go onto every routine the text makes, so each can answer
    /// for them once the reading is over.
    globe: Option<Value>,
    born: Option<Value>,
    /// The name the dictionary gave itself, read off it when the text
    /// was built: it goes onto every routine the text makes beside the
    /// dictionary, so a later write into the dictionary does not move
    /// the module a routine already made answers to.
    framed_in: Option<Rc<str>>,
    /// Where the value a write is to put is already waiting, which a
    /// taking-apart sets before each of its places.
    waiting: Option<String>,
    /// How far a compound write steps what the place already holds,
    /// where no value follows the sign: what `++` and `--` mean.
    stepping: Option<i64>,
    /// Cells a step keeps its two values in: what the place held before
    /// it, and what it holds after.
    stood: Option<String>,
    stands: Option<String>,
    /// The routines the text declares as giving back a cell and not a
    /// copy, so that a call of one is known to have a cell to share.
    gives_back: HashSet<String>,
    /// The routines being built, the innermost last, so a word standing
    /// for the one a piece is written in knows which that is.
    naming: Vec<String>,
    /// For each routine being built, whether it was declared global
    /// where it was written, so that its full name starts afresh.
    named_afresh: Vec<bool>,
    /// How many of those were being named when the class now being read
    /// was entered: a full name goes on from there.
    named_before: usize,
    /// Whether each routine being built gives back a cell and not a
    /// copy, the innermost last, so what it answers with is made a cell
    /// where it should be.
    giving_cells: Vec<bool>,
    /// The class each parameter of the routine being read is written to
    /// take, gathered as the parameters are read and taken up by the
    /// routine they belong to.
    formal_kinds: Vec<Option<Rc<str>>>,
    taking: Option<Vec<char>>,
    tells_place: bool,
    generator_seen: bool,
    reading_yield: bool,
    forbids_await: bool,
    top_coroutine: bool,
    source_before: Option<(usize, usize, String)>,
    unsupported_place: bool,
    iteration_binding: Option<(String, usize)>,
    place_depth: usize,
    outside_lambda: Vec<String>,
    loop_depth: usize,
    range_end: Option<(usize, u32)>,
    annotation_sites: Vec<(String, usize)>,
    module_sites: Vec<(String, usize)>,
    declarations: Vec<(usize, String, bool)>,
}

pub struct Built {
    pub program: Rc<Routine>,
    /// The reference's warnings about how the text is written, with the
    /// row and column of each.
    pub warnings: Vec<(String, u32, usize)>,
    pub globals: Vec<String>,
    /// The names the outermost statements declared global, which text
    /// read into two dictionaries writes to the outer one.
    pub outer_aliases: Vec<String>,
    pub seen: HashMap<String, Signature>,
    /// Which parameters of which routines take a cell rather than a
    /// value, and which routines hand a cell back. Text read while the
    /// run goes is a piece of the same program and must know what the
    /// whole of it declared.
    pub shared_args: HashMap<String, Vec<bool>>,
    /// What each routine calls its parameters, so that a language may
    /// name the one it speaks of.
    pub arg_names: HashMap<String, Vec<String>>,
    pub gives_back: HashSet<String>,
    /// Global names the text itself bound, by writing to them at its own
    /// outermost level: an assignment, an `import`, a `def` or a `class`.
    /// A name only ever read there, such as a builtin reached by its bare
    /// word, is not among these, though it too gets a global slot.
    pub bound_globally: Vec<String>,
    native_exports: HashSet<String>,
}

/// What a run of postfix words is part of, which says where it stops.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Body,
    Quoted,
    Single,
}

/// `before` is how many lines stand ahead of the program's own text,
/// which the host knows and a line named in a complaint must not count.
pub fn build(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32) -> Res<Built> {
    build_marking(tokens, table, seeded, assumed, strict, before, None, None, None, None, false, false)
}

/// The same, saying besides which row the reading had reached when it
/// stopped, for a language that tells such a stopping in its own words.
pub fn build_at(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32) -> Result<Built, (String, u32, bool, (usize, usize, u32))> {
    let at = std::cell::Cell::new(0u32);
    let hard = std::cell::Cell::new(false);
    let column = std::cell::Cell::new((1usize, 1usize, 0u32));
    build_marking(tokens, table, seeded, assumed, strict, before, None, Some((&at, &hard, &column)), None, None, false, false)
        .map_err(|said| (said, at.get(), hard.get(), column.get()))
}

/// The same, said besides which file the text came out of. Nothing but
/// text read in while the run was already going is built this way, so a
/// statement that means one thing in a program of its own and another
/// in a piece of a run in progress can tell the two apart.
pub fn build_from(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32, written_in: Option<Rc<str>>) -> Res<Built> {
    build_marking(tokens, table, seeded, assumed, strict, before, written_in, None, None, None, true, false)
}

/// The same, save that the text stands inside a routine already running:
/// the names that routine has are given here and a name the text reads
/// or writes means one of them. Names of its own go on the end, so the
/// frame it runs in need only be made longer. What the routine already
/// declared about cells is handed over too, since the text is a piece of
/// the same program.
pub fn build_within(
    tokens: &[Token],
    table: &Table,
    seeded: &[String],
    inside: &[String],
    knows: Knows,
    before: u32,
    within: Option<(String, Option<String>)>,
) -> Res<Built> {
    build_marking(tokens, table, seeded, HashMap::new(), true, before, None, None, Some((inside, knows)), within, true, false)
}

/// `build_within` and `build_from`, each saying besides which row the
/// reading had got to when it stopped: text read as the run goes is
/// told of by the line of its own that would not be read.
pub fn build_within_at(
    tokens: &[Token],
    table: &Table,
    seeded: &[String],
    inside: &[String],
    knows: Knows,
    before: u32,
    within: Option<(String, Option<String>)>,
    value_only: bool,
    origin: Option<Rc<str>>,
) -> Result<Built, (String, u32, (usize, usize, u32))> {
    let (at, hard) = (std::cell::Cell::new(0u32), std::cell::Cell::new(false));
    let column = std::cell::Cell::new((1usize, 1usize, 0u32));
    build_marking(tokens, table, seeded, HashMap::new(), true, before, origin, Some((&at, &hard, &column)), Some((inside, knows)), within, true, value_only).map_err(|said| (said, at.get(), column.get()))
}

pub fn build_module_position(tokens: &[Token], table: &Table, seeded: &[String], origin: Rc<str>) -> Result<Built, (String, u32, (usize, usize, u32))> {
    let line = std::cell::Cell::new(0);
    let fatal = std::cell::Cell::new(false);
    let span = std::cell::Cell::new((1, 1, 0));
    build_marking(tokens, table, seeded, HashMap::new(), false, 0, Some(origin), Some((&line, &fatal, &span)), None, None, false, false)
        .map_err(|message| (message, line.get(), span.get()))
}

pub fn build_from_at(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32) -> Result<Built, (String, u32)> {
    let (at, hard) = (std::cell::Cell::new(0u32), std::cell::Cell::new(false));
    let column = std::cell::Cell::new((1usize, 1usize, 0u32));
    build_marking(tokens, table, seeded, assumed, strict, before, None, Some((&at, &hard, &column)), None, None, true, false).map_err(|said| (said, at.get()))
}

/// Text handed over to be read while the run goes: as one expression
/// and nothing after it where it is to be weighed, else as statements;
/// said besides which file it came out of, and which builtin words are
/// to be read as names the program bound, in front of the builtins.
pub fn build_text(tokens: &[Token], table: &Table, seeded: &[String], before: u32, written_in: Option<Rc<str>>, globe: Option<Value>, born: Option<Value>, framed_in: Option<Rc<str>>, value_only: bool, shadowed: &[String], allow_top_await: bool, interactive: bool) -> Result<Built, (String, u32, (usize, usize, u32))> {
    let (at, hard) = (std::cell::Cell::new(0u32), std::cell::Cell::new(false));
    let column = std::cell::Cell::new((1usize, 1usize, 0u32));
    let mark = Some((&at, &hard, &column));
    let mut words = HashMap::new();
    let mut bound = shadowed.to_vec();
    let mut exports = HashSet::new();
    if table.flag("ext.stmt.function.closes_over") {
        let discovery = build_survey(tokens, table, seeded, HashMap::new(), true, before, written_in.clone(), globe.clone(), born.clone(), framed_in.clone(), mark, None, None, true, &mut words, true, value_only, shadowed, &HashSet::new(), allow_top_await, interactive).map_err(|said| (said, at.get(), column.get()))?;
        exports = discovery.native_exports;
        for name in discovery.bound_globally {
            if !bound.contains(&name) { bound.push(name); }
        }
    }
    build_survey(tokens, table, seeded, HashMap::new(), true, before, written_in, globe, born, framed_in, mark, None, None, true, &mut words, false, value_only, &bound, &exports, allow_top_await, interactive).map_err(|said| (said, at.get(), column.get()))
}

type Knows<'w> = (&'w HashMap<String, Vec<bool>>, &'w HashMap<String, Vec<String>>, &'w HashSet<String>);
type Within<'w> = (&'w [String], Knows<'w>);

fn build_marking(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32, written_in: Option<Rc<str>>, mark: Option<(&std::cell::Cell<u32>, &std::cell::Cell<bool>, &std::cell::Cell<(usize, usize, u32)>)>, within: Option<Within>, standing_in: Option<(String, Option<String>)>, read_in: bool, value_only: bool) -> Res<Built> {
    let mut words = HashMap::new();
    let (program_names, exports) = if table.flag("ext.stmt.function.closes_over") {
        let pass = build_survey(tokens, table, seeded, assumed.clone(), strict, before, written_in.clone(), None, None, None, mark, within, standing_in.clone(), read_in, &mut words, true, value_only, &[], &HashSet::new(), false, false)?;
        (pass.bound_globally, pass.native_exports)
    } else { (Vec::new(), HashSet::new()) };
    build_survey(tokens, table, seeded, assumed, strict, before, written_in, None, None, None, mark, within, standing_in, read_in, &mut words, false, value_only, &program_names, &exports, false, false)
}

/// Every name a `global` statement names anywhere in this text, however
/// deep the routine that says it stands, found by a plain walk of the
/// tokens rather than a read of the statements themselves: what is
/// wanted is only which names the text as a whole has ever declared
/// `global`, not where, so a walk that never enters or leaves a scope
/// answers it well enough.
fn text_wide_globals(tokens: &[Token], table: &Table) -> Vec<String> {
    let sep = table.single("syntax.call.separator");
    let mut names = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].shape == Shape::Bare && table.spells("ext.stmt.global", &tokens[i].lexeme) {
            i += 1;
            loop {
                match tokens.get(i) {
                    Some(t) if t.shape == Shape::Bare => {
                        if !names.iter().any(|n| n == &t.lexeme) { names.push(t.lexeme.clone()); }
                        i += 1;
                    }
                    _ => break,
                }
                match (sep, tokens.get(i)) {
                    (Some(s), Some(t)) if t.shape == Shape::Sign && t.lexeme == s => i += 1,
                    _ => break,
                }
            }
            continue;
        }
        i += 1;
    }
    names
}

pub(crate) fn member_spelling(class: &str, ident: &str) -> String {
    if ident.starts_with("__") && !ident.ends_with("__") && !ident.contains('.') {
        let stem = class.trim_start_matches('_');
        if !stem.is_empty() { return ["_", stem, ident].concat(); }
    }
    ident.to_owned()
}

/// Give each class suite its lexical names before collecting scope bindings.
/// An inner suite replaces the enclosing prefix, including in nested functions.
fn class_spellings(input: &[Token], table: &Table) -> Vec<Token> {
    let mut output = input.to_vec();
    if !table.has_any("ext.stmt.class.detail.slots") { return output; }
    let mut suites = Vec::new();
    for (index, token) in input.iter().enumerate() {
        if token.shape != Shape::Bare || !table.spells("ext.stmt.class", &token.lexeme) { continue; }
        let Some(class) = input.get(index + 1).filter(|t| t.shape == Shape::Bare) else { continue };
        let mut nesting: i32 = 0;
        let mut colon = None;
        for (offset, part) in input.iter().enumerate().skip(index + 2) {
            if matches!(part.shape, Shape::LineEnd | Shape::Finish) { break; }
            if part.shape != Shape::Sign { continue; }
            if ["(", "[", "{"].contains(&part.lexeme.as_str()) { nesting += 1; }
            if [")", "]", "}"].contains(&part.lexeme.as_str()) { nesting -= 1; }
            if nesting == 0 && table.spells("block.intro", &part.lexeme) { colon = Some(offset); break; }
        }
        let Some(mark) = colon else { continue };
        let start = (mark + 1..input.len()).find(|&i| input[i].shape != Shape::LineEnd).unwrap_or(input.len());
        let block = input.get(start).map_or(false, |t| t.shape == Shape::Open);
        let mut level = 0i32;
        let mut stop = input.len();
        for (i, item) in input.iter().enumerate().skip(start) {
            if item.shape == Shape::Open { level += 1; }
            if item.shape == Shape::Close { level -= 1; }
            if (item.shape == Shape::Close && level <= 0) || item.shape == Shape::Finish
                || (!block && item.shape == Shape::LineEnd) { stop = i; break; }
        }
        suites.push((start, stop, class.lexeme.as_str()));
    }
    for (begin, end, owner) in suites {
        for offset in begin..end {
            if input[offset].shape == Shape::Bare {
                // A name the table spells for a builtin is that builtin
                // in a class body as out of one: the private-name
                // mangling does not reach it.
                let word = &input[offset].lexeme;
                let named = word.starts_with("__") && !word.ends_with("__")
                    && crate::table::BUILTIN_LABELS.iter().any(|(label, _)| table.spells(label, word));
                if !named {
                    output[offset].lexeme = member_spelling(owner, word);
                }
            }
        }
    }
    output
}

fn build_survey(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32, written_in: Option<Rc<str>>, globe: Option<Value>, born: Option<Value>, framed_in: Option<Rc<str>>, mark: Option<(&std::cell::Cell<u32>, &std::cell::Cell<bool>, &std::cell::Cell<(usize, usize, u32)>)>, within: Option<Within>, standing_in: Option<(String, Option<String>)>, read_in: bool, words: &mut HashMap<usize, ScopeWords>, survey: bool, value_only: bool, shadowed: &[String], exports: &HashSet<String>, allow_top_await: bool, interactive: bool) -> Res<Built> {
    let original_words = tokens;
    let names_in_classes = class_spellings(tokens, table);
    let tokens = names_in_classes.as_slice();
    let mut beginnings = seeded.to_vec();
    for word in table.strings("ext.builtin.exceptions") {
        if !beginnings.contains(word) { beginnings.push(word.clone()); }
    }
    let top = Layer { async_walk_seen: false, permits_async: allow_top_await, gathering_kind: None, expression_targets: Vec::new(), comprehension: false, borrowed: Vec::new(), class_borrowed: Vec::new(), reaching: Vec::new(), holds: Holds::Every, idents: beginnings, formals: Vec::new(), formal_slots: Vec::new(), rpn: false, aliases: Vec::new(), encountered: Vec::new() };
    let (mut shared_args, mut arg_names, mut gives_back) = shared_parameters(tokens, table);
    let mut layers = vec![top];
    if let Some((inside, (args, spellings, backs))) = within {
        for (named, marks) in args {
            shared_args.entry(named.clone()).or_insert_with(|| marks.clone());
        }
        for (named, spelt) in spellings {
            arg_names.entry(named.clone()).or_insert_with(|| spelt.clone());
        }
        gives_back.extend(backs.iter().cloned());
        layers.push(Layer { async_walk_seen: false, permits_async: false, gathering_kind: None, expression_targets: Vec::new(), comprehension: false, borrowed: Vec::new(), class_borrowed: Vec::new(), reaching: Vec::new(), holds: Holds::Fresh, idents: inside.to_vec(), formals: Vec::new(), formal_slots: Vec::new(), rpn: false, aliases: Vec::new(), encountered: Vec::new() });
    }
    let outer_layers = layers.len();
    if read_in && outer_layers > 1 {
        // Text handed over while the run goes stands one layer short
        // of the outermost, that layer standing for the frame around
        // it: a name any routine written in the text declares `global`
        // means the very outermost binding there too, as it does
        // anywhere else `global` is said, so the text's own top level
        // -- reading or writing the name directly, never inside a
        // routine of its own -- has to reach the same place, exactly
        // as the reference has the whole of what was handed over
        // agree on the one binding once any routine in it has said so.
        for name in text_wide_globals(tokens, table) {
            if !layers[0].aliases.iter().any(|(n, _)| *n == name) {
                layers[0].aliases.push((name.clone(), name));
            }
        }
    }
    let mut r = Builder { syntax_try_nesting: 0, finally_nesting: 0, in_lazy_from: false, module_sites: Vec::new(), annotation_sites: Vec::new(), declarations: Vec::new(), class_met: Vec::new(), importing: false, asynchronous: false, in_coroutine: false, coroutine_next: false, pattern_kinds: Vec::new(), warnings: Vec::new(), loop_depth: 0, range_end: None, kind_mark: None, surveyed: words.clone(), survey, class_bindings: Vec::new(), class_globals: Vec::new(), under_way: Vec::new(), receiver: None, outside_lambda: Vec::new(), within: standing_in, bags: HashMap::new(), shared_args, arg_names, gives_back, stopped_fatally: false, declared_at: 0, carrying: Vec::new(), pending_types: Vec::new(), noted_when_read: Vec::new(), table, forks: Vec::new(), tokens, original_words, pos: 0, layers, read_in, interactive, outer_layers, spoken_for: Vec::new(), gensyms: 0, gather_names: Vec::new(), presumed: assumed, seen: HashMap::new(), strict, statics: Vec::new(), also_property: Vec::new(), before, past_library: false, named_in_program: shadowed.to_vec(), native_exports: exports.clone(), written_in, globe, born, framed_in, waiting: None, stepping: None, stood: None, stands: None, naming: Vec::new(), named_afresh: Vec::new(), named_before: 0, giving_cells: Vec::new(), formal_kinds: Vec::new(), taking: None,
        generator_seen: false,
        top_coroutine: false,
        reading_yield: false, forbids_await: false, place_depth: 0,
        source_before: None,
        unsupported_place: false,
        iteration_binding: None,
        tells_place: ["ext.system.complaint.warning", "ext.system.complaint.notice", "ext.system.complaint.deprecated", "ext.system.complaint.fatal"]
            .iter()
            .any(|key| table.single(key).is_some()) || table.flag("ext.system.source.marked") };
    let body = if value_only {
        // One expression, with line ends about it and nothing else.
        r.skip_line_ends();
        let value = match r.comma_expression(false) {
            Ok(value) => value,
            Err(said) => {
                if let Some((line, fatal, col)) = mark { line.set(r.look().row); col.set(r.error_columns()); fatal.set(r.stopped_fatally); }
                return Err(said);
            }
        };
        r.skip_line_ends();
        if !r.exhausted() {
            if let Some((line, _, col)) = mark { line.set(r.look().row); col.set(r.error_columns()); }
            if r.pos > 0 && r.tokens[r.pos - 1].shape == Shape::Quote
                && r.look().shape == Shape::Bare
                && r.tokens[r.pos..].iter().take_while(|token| token.row == r.look().row)
                    .any(|token| token.shape == Shape::Quote) {
                return Err(String::from("SyntaxError: invalid syntax. Is this intended to be part of the string?"));
            }
            return Err(format!("Unexpected '{}'", r.look().lexeme));
        }
        r.warnings_in_statement(0);
        if table.has_any("ext.system.syntax_warnings") {
            for index in 1..r.tokens.len() {
                let left = &r.tokens[index - 1];
                let right = &r.tokens[index];
                if left.shape == Shape::Numeral && right.shape == Shape::Bare && left.row == right.row
                    && left.column + left.lexeme.chars().count() == right.column
                    && ["and", "or", "in", "not", "if", "else", "for", "is"].contains(&right.lexeme.as_str()) {
                    let radix_word = if left.lexeme.starts_with("0x") { "hexadecimal" }
                        else if left.lexeme.starts_with("0o") { "octal" }
                        else if left.lexeme.starts_with("0b") { "binary" } else { "decimal" };
                    let detail = (format!("invalid {radix_word} literal"), left.row, right.column);
                    if !r.warnings.contains(&detail) { r.warnings.push(detail); }
                }
            }
        }
        value
    } else if table.rpn {
        let (mut stmts, rest) = match r.rpn_body(&[], Mode::Body) {
            Ok(got) => got,
            Err(said) => {
                if let Some((mark, hard, column)) = mark {
                    mark.set(r.look().row);
                    column.set(r.error_columns());
                    hard.set(r.stopped_fatally);
                }
                return Err(said);
            }
        };
        if !r.exhausted() {
            if let Some((mark, hard, column)) = mark {
                mark.set(r.look().row);
                    column.set(r.error_columns());
                hard.set(r.stopped_fatally);
            }
            return Err(format!("Unexpected '{}'", r.look().lexeme));
        }
        stmts.extend(rest.into_iter().filter(|n| !inert(n)));
        sequence(stmts)
    } else {
        // A top-level function may be bound ahead of everything else, so a
        // call written above it finds it (ext.stmt.function.hoisted).
        let mut stmts = Vec::new();
        let mut ahead = Vec::new();
        let mut opening = true;
        r.skip_line_ends();
        while !r.exhausted() {
            let defines = table.flag("ext.stmt.function.hoisted") && r.key("stmt.function");
            let own_line = r.look().row > before;
            if opening && own_line && before != 0 && within.is_none() && !read_in && !survey
                && table.flag("ext.syntax.names.shadow_builtins") {
                for word in r.named_in_program.clone() {
                    if let Some(operation) = table.prims.get(&word).copied() {
                        let address = r.global_address(&word);
                        let native = constant(Value::Intrinsic(operation, Rc::from(word.as_str())));
                        stmts.push(Form::Write(address, Box::new(native)));
                    }
                }
            }
            // Where the reading stops, the row it had reached is kept,
            // so a language with a word for such a stopping names it.
            let stmt = match r.stmt() {
                Ok(stmt) => stmt,
                Err(said) => {
                    if let Some((mark, hard, column)) = mark {
                        mark.set(r.look().row);
                    column.set(r.error_columns());
                        hard.set(r.stopped_fatally);
                    }
                    return Err(said);
                }
            };
            // Text standing alone as the first statement of a program
            // is the program's own documentation, kept under the name
            // the language gives it (ext.system.module.doc).
            if opening && own_line && within.is_none() {
                let first = match &stmt { Form::OnLine(_, inner) => inner.as_ref(), other => other };
                if let Form::Const(Value::Text(said)) = first {
                    for name in table.strings("ext.system.module.doc") {
                        let slot = r.global_address(name);
                        stmts.push(Form::Write(slot, Box::new(Form::Const(Value::Text(said.clone())))));
                    }
                }
            }
            if own_line { opening = false; }
            if defines {
                ahead.push(stmt);
            } else {
                stmts.push(stmt);
            }
            r.skip_line_ends();
        }
        ahead.extend(stmts);
        sequence(ahead)
    };
    let body = if r.module_sites.is_empty() { body } else {
        r.annotation_sites = std::mem::take(&mut r.module_sites);
        let annotation = r.build_annotator()?.expect("module annotations");
        let name = annotation.ident.clone();
        let bind = r.write(&name, constant(Value::Routine(annotation)));
        sequence(vec![body, bind])
    };
    // What the reading itself remarked on was noticed before a single
    // statement ran, so it is said ahead of the whole body.
    let body = if r.noted_when_read.is_empty() {
        body
    } else {
        let mut first = std::mem::take(&mut r.noted_when_read);
        first.push(body);
        sequence(first)
    };
    words.extend(r.surveyed.clone());
    let top = r.layers.pop().unwrap();
    // Where the text stands inside a routine, the layer just popped is
    // that routine's and the one under it holds the globals.
    let outer_aliases: Vec<String> = top.aliases.iter().map(|(name, _)| name.clone()).collect();
    let globals = match r.layers.pop() {
        Some(under) => under.idents,
        None => top.idents.clone(),
    };
    let program = Routine { class_namespace: None, annotator: None, literals: Vec::new(), referenced: Vec::new(), locals: Vec::new(), flags: 0, lineless: false, qualification: String::new(), doc: None, generator: r.top_coroutine, local_defaults: Vec::new(), gather_from: None, ident: "<program>".into(), least: 0, formals: Vec::new(), formal_kinds: Vec::new(), taking: None, formal_slots: Vec::new(), idents: top.idents, reaching: top.reaching, frameless: r.top_coroutine, written_in: r.written_in.clone(), within: None, declared_on: 0, type_params: Vec::new(), globe: r.globe.clone(), born: r.born.clone(), framed_in: r.framed_in.clone(), traps: Traps::Naught, carried: Vec::new(), body };
    Ok(Built { program: Rc::new(program), warnings: r.warnings, globals, outer_aliases, seen: r.seen, shared_args: r.shared_args, arg_names: r.arg_names, gives_back: r.gives_back, bound_globally: r.named_in_program, native_exports: r.native_exports })
}

/// Which parameters of each program are written with the reference sign.
/// A call must know before it works out its arguments, and a program may
/// be called above where it is written, so the tokens are read first.
fn shared_parameters(tokens: &[Token], table: &Table) -> (HashMap<String, Vec<bool>>, HashMap<String, Vec<String>>, HashSet<String>) {
    let mut found = HashMap::new();
    let mut spelled = HashMap::new();
    let mut gives = HashSet::new();
    let (Some(mark), Some(open)) = (table.single("ext.op.reference"), table.single("syntax.call.open")) else { return (found, spelled, gives) };
    let close = table.single("syntax.call.close").unwrap_or(")");
    let sep = table.single("syntax.call.separator");
    let is = |t: &Token, text: &str| t.shape == Shape::Sign && t.lexeme == text;
    let mut i = 0;
    while i + 2 < tokens.len() {
        let word = &tokens[i];
        if word.shape != Shape::Bare || !table.spells("stmt.function", &word.lexeme) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let hands_back = is(&tokens[j], mark);
        if hands_back {
            j += 1;
        }
        if tokens[j].shape != Shape::Bare || !is(&tokens[j + 1], open) {
            i += 1;
            continue;
        }
        let name = tokens[j].lexeme.clone();
        if hands_back {
            gives.insert(name.clone());
        }
        j += 2;
        let (mut marks, mut shares, mut any, mut defaulting, mut depth) = (Vec::new(), false, false, false, 1usize);
        let (mut spellings, mut spelling) = (Vec::new(), String::new());
        while j < tokens.len() {
            let p = &tokens[j];
            if is(p, open) {
                depth += 1;
            } else if is(p, close) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            } else if depth == 1 {
                any = true;
                if sep.map_or(false, |s| is(p, s)) {
                    marks.push(shares);
                    spellings.push(std::mem::take(&mut spelling));
                    shares = false;
                    defaulting = false;
                } else if p.shape == Shape::Sign && table.spells("stmt.assign", &p.lexeme) {
                    defaulting = true;
                } else if !defaulting && is(p, mark) {
                    shares = true;
                } else if !defaulting && spelling.is_empty() && p.shape == Shape::Bare {
                    spelling = p.lexeme.clone();
                }
            }
            j += 1;
        }
        if any {
            marks.push(shares);
            spellings.push(spelling);
        }
        spelled.insert(name.clone(), spellings);
        found.insert(name, marks);
        i = j;
    }
    (found, spelled, gives)
}

// ---------- node builders


/// What a class body gathers as it is read: its properties and how far
/// each is reached from, the values it keeps for itself, its constants
/// and its methods.
struct Members {
    fields: Vec<(String, Form)>,
    shared: Vec<(String, Form)>,
    constants: Vec<(String, Form)>,
    methods: Vec<(String, Rc<Routine>)>,
    reaches: Vec<Reach>,
}

/// What a routine written where a value stands is called, being
/// bound to no name of its own.
const ANONYMOUS: &str = "{closure}";

/// Whether a kind written before a parameter names a class: a word the
/// language has a kind of its own for does not, nor does one it lists as
/// naming none.
fn names_a_class(table: &Table, word: &str) -> bool {
    kind_made(table, word).is_none() && !table.spells("ext.system.kind.loose", word)
}

/// The word this language names a value's kind by, the shorter one where
/// it has one.
fn kind_word(table: &Table, v: &Value) -> String {
    use crate::data::Kind;
    let order = [Kind::Whole, Kind::Fraction, Kind::Decimal, Kind::Chars, Kind::Truth, Kind::Vector, Kind::Nothing];
    let Some(kind) = v.kind() else { return "value".to_string() };
    let at = order.iter().position(|k| *k == kind);
    match at.and_then(|i| table.strings("ext.system.kind.brief").get(i)).filter(|word| *word != "-") {
        Some(word) => word.clone(),
        None => crate::exec::KIND_LABELS
            .iter()
            .find(|(_, k)| *k == kind)
            .and_then(|(label, _)| table.single(label))
            .unwrap_or("value")
            .to_string(),
    }
}

fn constant(v: Value) -> Form {
    Form::Const(v)
}

fn prim_call(op: Prim, args: Vec<Form>) -> Form {
    let name: &str = match op {
        Prim::Seq => "last",
        Prim::Choose => "if",
        Prim::Both => "and",
        Prim::Either => "or",
        Prim::Yield => "return",
        Prim::Leave => "break",
        Prim::Resume => "continue",
        _ => "",
    };
    let two = matches!(op, Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power
        | Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::Join);
    if two && args.len() == 2 {
        let mut it = args.into_iter();
        let (a, b) = (it.next().unwrap(), it.next().unwrap());
        return Form::Dyad { op, name: Rc::from(name), a: Input::of(a), b: Input::of(b) };
    }
    Form::Apply(Callee::Prim(op, Rc::from(name)), args)
}

/// Every look inside a value being asked about becomes a glance, which
/// answers nothing rather than minding that nothing is there.
fn glancing(form: Form) -> Form {
    match form {
        Form::Apply(Callee::Prim(Prim::At, name), args) => {
            Form::Apply(Callee::Prim(Prim::Glance, name), args.into_iter().map(glancing).collect())
        }
        other => other,
    }
}

/// An index chain written on a bare name: the name, the keys from the
/// outside in, and whether the write lands after the last place rather
/// than at one of them. A chain standing on anything but a name is
/// given back untouched, since it is written into another way.
/// A chain of looks standing on something other than a bare name: what
/// it stands on, the keys from the outside in, and whether the write
/// lands after the last place. A chain on a bare name is given back
/// untouched, since that one is written into a shorter way.
fn footing_apart(form: Form) -> Result<(Form, Vec<Form>, bool), Form> {
    let (mut walk, after) = match form {
        Form::Apply(Callee::Prim(Prim::AtEnd, name), mut args) if args.len() == 1 => match args.pop() {
            Some(only) => (only, true),
            None => return Err(Form::Apply(Callee::Prim(Prim::AtEnd, name), Vec::new())),
        },
        other => (other, false),
    };
    let mut keys = Vec::new();
    let mut peeled: Vec<Form> = Vec::new();
    loop {
        match walk {
            Form::Apply(Callee::Prim(Prim::At, name), mut args) if args.len() == 2 => {
                let key = args.pop().expect("the key");
                let held = args.pop().expect("what holds it");
                keys.push(key);
                peeled.push(Form::Apply(Callee::Prim(Prim::At, name), Vec::new()));
                walk = held;
            }
            // A bare name is written into the shorter way; anything
            // else stands under the chain and is written back into.
            Form::Read(slot) => return Err(put_back(Form::Read(slot), peeled, keys, after)),
            other if keys.is_empty() => return Err(put_back(other, peeled, keys, after)),
            other => {
                keys.reverse();
                return Ok((other, keys, after));
            }
        }
    }
}

/// A chain taken apart and put back as it was.
fn put_back(mut back: Form, peeled: Vec<Form>, mut keys: Vec<Form>, after: bool) -> Form {
    for shell in peeled.into_iter().rev() {
        let Form::Apply(callee, _) = shell else { unreachable!("a look") };
        back = Form::Apply(callee, vec![back, keys.pop().expect("its key")]);
    }
    match after {
        true => prim_call(Prim::AtEnd, vec![back]),
        false => back,
    }
}

fn chain_apart(form: Form) -> Result<(String, Vec<Form>, bool), Form> {
    let (mut walk, after) = match form {
        Form::Apply(Callee::Prim(Prim::AtEnd, name), mut args) if args.len() == 1 => match args.pop() {
            Some(only) => (only, true),
            None => return Err(Form::Apply(Callee::Prim(Prim::AtEnd, name), Vec::new())),
        },
        other => (other, false),
    };
    let mut keys = Vec::new();
    let mut peeled: Vec<Form> = Vec::new();
    loop {
        match walk {
            Form::Apply(Callee::Prim(Prim::At, name), mut args) if args.len() == 2 => {
                let key = args.pop().expect("the key");
                let held = args.pop().expect("what holds it");
                keys.push(key);
                peeled.push(Form::Apply(Callee::Prim(Prim::At, name), Vec::new()));
                walk = held;
            }
            Form::Read(slot) => {
                let enough = keys.len() > 1 || (after && !keys.is_empty());
                if enough {
                    keys.reverse();
                    return Ok((slot.ident.to_string(), keys, after));
                }
                // Put the chain back as it was for the arms that take it.
                let mut back = Form::Read(slot);
                for shell in peeled.into_iter().rev() {
                    let Form::Apply(callee, _) = shell else { unreachable!("a look") };
                    back = Form::Apply(callee, vec![back, keys.pop().expect("its key")]);
                }
                if after {
                    back = prim_call(Prim::AtEnd, vec![back]);
                }
                return Err(back);
            }
            other => {
                let mut back = other;
                for shell in peeled.into_iter().rev() {
                    let Form::Apply(callee, _) = shell else { unreachable!("a look") };
                    back = Form::Apply(callee, vec![back, keys.pop().expect("its key")]);
                }
                if after {
                    back = prim_call(Prim::AtEnd, vec![back]);
                }
                return Err(back);
            }
        }
    }
}

/// The making a word calls for, by the word a language asks a value's
/// kind with or by the shorter word it complains with. Nothing where
/// the word names no kind of that language's.
fn kind_made(table: &Table, word: &str) -> Option<Prim> {
    const MAKINGS: [Prim; 7] = [Prim::AsWhole, Prim::AsDecimal, Prim::AsDecimal, Prim::AsChars, Prim::AsTruth, Prim::AsVector, Prim::AsNothing];
    if let Some(at) = crate::exec::KIND_LABELS.iter().position(|(label, _)| table.single(label) == Some(word)) {
        return MAKINGS.get(at).copied();
    }
    let at = table.strings("ext.system.kind.brief").iter().position(|n| n != "-" && n == word)?;
    MAKINGS.get(at).copied()
}

fn invoke(program: Form, args: Vec<Form>) -> Form {
    Form::Apply(Callee::Code(Box::new(program)), args)
}

/// Statements in order: one is itself, several are a call of `last`.
fn sequence(mut items: Vec<Form>) -> Form {
    match items.len() {
        0 => constant(Value::Nil),
        1 => items.pop().unwrap(),
        _ => prim_call(Prim::Seq, items),
    }
}

/// Whether an operation can take this side as it stands, wanting no
/// holding place of its own for it: a name or a plain value is fetched
/// by the operation itself, so nothing at all happens before it.
fn as_it_stands(node: &Form) -> bool {
    matches!(node, Form::Read(_)) || matches!(node, Form::Const(v) if !matches!(v, Value::Routine(_)))
}

fn inert(node: &Form) -> bool {
    match node {
        Form::Const(_) | Form::Read(_) => true,
        Form::Dyad { a, b, .. } => [a, b].iter().all(|o| match o {
            Input::Form(n) => inert(n),
            _ => true,
        }),
        Form::Apply(Callee::Prim(op, _), args) => {
            !matches!(op, Prim::Echo | Prim::Say | Prim::Out | Prim::Tell | Prim::Dump | Prim::Define | Prim::Gather | Prim::Raise | Prim::External | Prim::Append | Prim::Replace | Prim::Front | Prim::Yield | Prim::Leave | Prim::Resume | Prim::Choose | Prim::Both | Prim::Either | Prim::Seq)
                && args.iter().all(inert)
        }
        _ => false,
    }
}

fn yields_value(node: &Form) -> bool {
    match node {
        Form::Apply(Callee::Prim(Prim::Seq, _), args) => args.last().map_or(false, yields_value),
        Form::Apply(Callee::Prim(Prim::Yield, _), args) => !args.is_empty(),
        Form::Apply(Callee::Prim(Prim::Choose, _), args) => args[1..].iter().any(|arm| match arm {
            Form::Const(Value::Routine(p)) => yields_value(&p.body),
            _ => false,
        }),
        _ => false,
    }
}

impl<'a> Builder<'a> {
    // ---------- tokens

    fn error_columns(&self) -> (usize, usize, u32) {
        if let Some((end, row)) = self.range_end { return (self.look().column, end, row); }
        let token = self.look();
        let finish = match token.shape {
            Shape::Finish | Shape::LineEnd | Shape::Close => token.column,
            _ if token.end_column > 0 => token.end_column,
            _ => token.column + token.lexeme.chars().count(),
        };
        (token.column, finish, token.row.max(token.end_row))
    }

    fn look(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn glance(&self, n: usize) -> &Token {
        &self.tokens[(self.pos + n).min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let t = self.look().clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn exhausted(&self) -> bool {
        self.look().shape == Shape::Finish
    }

    fn sign(&self, s: &str) -> bool {
        self.look().shape == Shape::Sign && self.look().lexeme == s
    }

    fn lexeme_of(&self, s: &str) -> bool {
        matches!(self.look().shape, Shape::Sign | Shape::Bare) && self.look().lexeme == s
    }

    fn on_any(&self, key: &str) -> bool {
        self.table.strings(key).iter().any(|w| self.lexeme_of(w))
    }

    fn key(&self, key: &str) -> bool {
        self.look().shape == Shape::Bare && self.table.spells(key, &self.look().lexeme)
    }

    fn on_stmt_end(&self) -> bool {
        let t = self.look();
        t.shape == Shape::LineEnd || (t.shape == Shape::Sign && self.table.separates(&t.lexeme))
    }

    /// Whether a block opens here, past any line ends before it, so
    /// that a body written on the line after a name is still a body.
    fn block_opens_ahead(&self) -> bool {
        let mut ahead = 0;
        while self.glance(ahead).shape == Shape::LineEnd {
            ahead += 1;
        }
        let t = self.glance(ahead);
        t.shape == Shape::Sign && self.table.strings("block.open").iter().any(|o| *o == t.lexeme)
    }

    fn skip_line_ends(&mut self) {
        while !self.exhausted() && self.on_stmt_end() {
            self.advance();
        }
    }

    fn need_sign(&mut self, s: &str, why: &str) -> Res<()> {
        if self.sign(s) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", s, why, self.look().lexeme))
        }
    }

    fn need_word(&mut self, why: &str) -> Res<String> {
        if self.table.has_any("ext.builtin.exceptions.syntax") && ["literal.true", "literal.false", "literal.null"].iter().any(|label| self.key(label)) {
            return Err(String::from("SyntaxError: invalid syntax"));
        }
        if self.look().shape == Shape::Bare {
            Ok(self.advance().lexeme)
        } else {
            Err(format!("Expected identifier {}, got '{}'", why, self.look().lexeme))
        }
    }

    fn need_lexeme(&mut self, s: &str) -> Res<()> {
        if self.lexeme_of(s) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", s, self.look().lexeme))
        }
    }

    fn need_closer(&mut self) -> Res<()> {
        if self.on_any("block.close") {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", self.table.single("block.close").unwrap_or("end"), self.look().lexeme))
        }
    }

    // ---------- names

    fn global_at(&mut self, name: &str) -> usize {
        let top = &mut self.layers[0];
        match top.idents.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                top.idents.push(name.to_string());
                top.idents.len() - 1
            }
        }
    }

    /// The binding a read reaches: the nearest owner that has the name,
    /// stopping at a function; else the global.
    /// The global address of a name, from wherever the reader stands.
    fn global_address(&mut self, name: &str) -> Address {
        let up = self.layers[1..].iter().filter(|s| s.holds != Holds::Nothing).count();
        let at = self.global_at(name);
        Address { ident: Rc::from(name), up, at, fallback: None }
    }

    /// The global a name stands for in the function around, if a
    /// `global` or `static` statement bound it, or in the class body
    /// standing here, at this very depth, if one of its own `global`
    /// statements did: that one reaches no further, so a routine
    /// nested in the body -- standing one or more layers deeper -- and
    /// the layer once the body is behind both fall through to whatever
    /// the function around held before the body was ever entered.
    fn aliased(&mut self, name: &str) -> Option<Address> {
        if let Some((depth, names)) = self.class_globals.last() {
            if *depth == self.layers.len() && names.iter().any(|n| n == name) {
                let mut slot = self.global_address(name);
                slot.ident = Rc::from(name);
                return Some(slot);
            }
        }
        let owner = self.layers.iter().rev().find(|s| s.holds == Holds::Every)?;
        let target = owner.aliases.iter().rev().find(|(n, _)| n == name)?.1.clone();
        let mut slot = self.global_address(&target);
        // The binding keeps the name the source calls it by, so that a
        // read of it can be turned back into a write of the same one and
        // a complaint about it says what a reader would recognise.
        slot.ident = Rc::from(name);
        Some(slot)
    }

    fn lexical_address(&mut self, name: &str, borrowed: bool) -> Option<Address> {
        let mut distance = 0;
        let owner = self.layers.iter().rposition(|s| s.holds == Holds::Every).unwrap_or(0);
        // The functions the walk goes out through before it finds the
        // binding, each with how far it stood from where the walk began.
        // Every one of them reaches the name, so every one of them has to
        // be told of it: what a function can see is not only what it
        // keeps, and a function in the middle of the chain keeps nothing
        // of a name it only passes along.
        let mut through: Vec<(usize, usize)> = Vec::new();
        for index in (1..self.layers.len()).rev() {
            let layer = &self.layers[index];
            if layer.holds == Holds::Nothing { continue; }
            // Text read as standing inside a routine knows that
            // routine's names at its own top level alone: a routine
            // written in the text reaches the globals and none of them,
            // as the reference has it.
            let set_aside = self.read_in && index < self.outer_layers && index + 1 < self.layers.len();
            if !set_aside && !(borrowed && index == owner) {
                if layer.aliases.iter().any(|pair| pair.0 == name) { return None; }
                if let Some(at) = layer.idents.iter().rposition(|word| word == name) {
                    let slot = Address { ident: Rc::from(name), up: distance, at, fallback: None };
                    for (which, stood) in through {
                        let layer = &mut self.layers[which];
                        if layer.reaching.iter().any(|held| held.ident.as_ref() == name) { continue; }
                        layer.reaching.push(Address { up: distance - stood, ..slot.clone() });
                    }
                    return Some(slot);
                }
            }
            if layer.holds == Holds::Every { through.push((index, distance)); }
            distance += 1;
        }
        None
    }

    fn address_to_read(&mut self, name: &str) -> Address {
        self.mark_met(name, MET_READ);
        let private = self.gather_names.iter().rfind(|pair| pair.0 == name).map(|pair| pair.1.to_string());
        let name = private.as_deref().unwrap_or(name);
        if let Some(slot) = self.aliased(name) {
            return slot;
        }
        if self.table.flag("ext.stmt.function.closes_over") && !self.survey {
            if let Some(slot) = self.lexical_address(name, false) { return slot; }
            return self.global_address(name);
        }
        let frameless = |holds: Holds| holds == Holds::Nothing;
        let mut depth = 0;
        let mut found: Option<(usize, usize, usize)> = None;
        // Where the walk stops at a function, that function is where a
        // name of its own would be kept, and how far up it stands.
        let mut routine: Option<(usize, usize)> = None;
        for (i, scope) in self.layers.iter().enumerate().rev() {
            if let Some(index) = scope.idents.iter().rposition(|n| n == name).filter(|_| scope.holds != Holds::Nothing) {
                found = Some((i, depth, index));
                break;
            }
            if scope.holds == Holds::Every && i != 0 {
                routine = Some((i, depth));
                break;
            }
            if !frameless(scope.holds) {
                depth += 1;
            }
        }
        let global = self.global_at(name);
        // A variable a function does no more than read is kept among
        // that function's own names all the same
        // (ext.stmt.function.own_names). Nothing is ever written there
        // by the function itself, and a name holding nothing is read
        // from the outermost binding as before, so no program can tell
        // the difference; what the room is for is text handed over
        // while the run goes, which is a piece of the function that
        // read it and needs somewhere in its frame to write. Only what
        // the language marks as a variable is kept: a bare word names a
        // constant or a class, and what the builder makes for itself is
        // none of the program's business.
        if let (None, Some((i, up))) = (found, routine) {
            let wears_mark = self.table.letter("identifier.variable_prefix").map_or(true, |mark| name.starts_with(mark));
            if wears_mark && self.table.flag("ext.stmt.function.own_names") {
                let own = &mut self.layers[i].idents;
                own.push(name.to_string());
                return Address { ident: Rc::from(name), up, at: own.len() - 1, fallback: Some(global) };
            }
        }
        match found {
            Some((i, depth, index)) if i != 0 => Address { ident: Rc::from(name), up: depth, at: index, fallback: Some(global) },
            Some((_, depth, index)) => Address { ident: Rc::from(name), up: depth, at: index, fallback: None },
            None => {
                let depth = self.layers[1..].iter().filter(|s| !frameless(s.holds)).count();
                Address { ident: Rc::from(name), up: depth, at: global, fallback: None }
            }
        }
    }

    /// The binding a write reaches: the nearest owner; a function or the
    /// top level makes the name if it has none, a block makes its own.
    fn address_to_write(&mut self, name: &str) -> Address {
        if !self.importing { self.mark_met(name, MET_WRITTEN); }
        // A name a class body knows is written into the place the body
        // keeps it in, as it is read out of there: the loops, imports
        // and handlers of a body bind members by ordinary writes.
        if let Some((depth, names)) = self.class_bindings.last() {
            if *depth == self.layers.len() {
                if let Some(slot) = names.get(name) { return slot.clone(); }
            }
        }
        let private = self.gather_names.iter().rfind(|pair| pair.0 == name).map(|pair| pair.1.to_string());
        let name = private.as_deref().unwrap_or(name);
        if self.table.flag("ext.stmt.function.closes_over") && private.is_none() && !name.starts_with('#') {
            let here = self.layers.iter().rposition(|l| l.holds == Holds::Every).unwrap_or(0);
            if self.layers[here].comprehension {
                let owner = (0..here).rev().find(|&i| self.layers[i].holds == Holds::Every && !self.layers[i].comprehension).unwrap_or(0);
                let target = self.layers[owner].aliases.iter().find(|pair| pair.0 == name).map(|pair| pair.1.clone());
                match target {
                    Some(global) => self.layers[here].aliases.push((name.to_string(), global)),
                    None if owner == 0 => self.layers[here].aliases.push((name.to_string(), name.to_string())),
                    None => {
                        let layer = &mut self.layers[owner];
                        if !layer.borrowed.iter().any(|n| n == name) && !layer.idents.iter().any(|n| n == name) { layer.idents.push(name.to_string()); }
                        self.layers[here].borrowed.push(name.to_string());
                    }
                }
            }
        }
        if let Some(slot) = self.aliased(name) {
            if self.past_library { self.native_exports.remove(name); }
            if self.past_library && !self.named_in_program.iter().any(|word| word == name) {
                self.named_in_program.push(name.to_owned());
            }
            return slot;
        }
        let is_borrowed = self.layers.iter().rev().find(|l| l.holds == Holds::Every).map_or(false, |l| l.borrowed.iter().any(|word| word == name));
        if is_borrowed && !self.survey && self.table.flag("ext.stmt.function.closes_over") {
            if let Some(slot) = self.lexical_address(name, true) { return slot; }
        }
        let depth = 0;
        let last = self.layers.len() - 1;
        for i in (0..=last).rev() {
            let scope = &mut self.layers[i];
            match scope.holds {
                // Makes no frame when it runs.
                Holds::Nothing => continue,
                Holds::Fresh | Holds::Every => {
                    let index = match scope.idents.iter().rposition(|n| n == name) {
                        Some(i) => i,
                        None => {
                            scope.idents.push(name.to_string());
                            scope.idents.len() - 1
                        }
                    };
                    if i == 0 && self.past_library { self.native_exports.remove(name); }
                    if i == 0 && self.past_library && !self.named_in_program.iter().any(|word| word == name) {
                        self.named_in_program.push(name.to_string());
                    }
                    return Address { ident: Rc::from(name), up: depth, at: index, fallback: None };
                }
            }
        }
        unreachable!("the top scope holds every name")
    }

    fn gensym(&mut self, what: &str) -> Address {
        self.gensyms += 1;
        let name = format!("#{}{}", what, self.gensyms);
        self.address_to_write(&name)
    }

    /// Whether the statements being read belong to a class body itself,
    /// not to a method within it nor to the program around it.
    /// The list a name met in the scope standing here is remembered
    /// in: the class body's own where the statements are a class
    /// body's, the layer's anywhere else.
    fn met_here(&mut self) -> &mut Vec<(String, u8)> {
        if self.in_class_body() { return self.class_met.last_mut().expect("the class body"); }
        &mut self.layers.last_mut().expect("a layer").encountered
    }

    /// Remember a name the scope standing here read, wrote or
    /// annotated. A `global` or `nonlocal` naming it after the fact is
    /// a fault, worded by what came first.
    fn mark_met(&mut self, name: &str, kind: u8) {
        if !self.table.has_any("ext.builtin.exceptions.syntax") || name.starts_with('#') { return; }
        let met = self.met_here();
        if kind == MET_READ || !met.iter().any(|(n, k)| n == name && *k == kind) { met.push((name.to_string(), kind)); }
    }

    /// Take back the last reading remembered for a name: the target of
    /// an annotation or of a write is no use of the name, however it
    /// was read to get there.
    fn unmet_last_read(&mut self, name: &str) {
        if !self.table.has_any("ext.builtin.exceptions.syntax") { return; }
        let met = self.met_here();
        if let Some(at) = met.iter().rposition(|(n, k)| n == name && *k == MET_READ) { met.remove(at); }
    }

    fn in_class_body(&self) -> bool {
        self.class_bindings.last().map_or(false, |(level, _)| *level == self.layers.len())
    }

    /// Whether the layer around the class body already holds this name
    /// `global` or `nonlocal`, the two declarations that carry a name
    /// past a class body to the place it named before the body was
    /// ever entered.
    fn declared_outside_class(&self, name: &str) -> bool {
        if let Some((depth, names)) = self.class_globals.last() {
            if *depth == self.layers.len() && names.iter().any(|named| named == name) { return true; }
        }
        self.layers.iter().rev().find(|layer| layer.holds == Holds::Every)
            .map_or(false, |layer| layer.aliases.iter().any(|(named, _)| named == name) || layer.borrowed.iter().any(|named| named == name))
    }

    /// The parts gathered so far of the class whose body is being read.
    fn parts(&mut self) -> &mut ClassParts {
        self.under_way.last_mut().expect("a class body under way")
    }

    /// A map built from the class body's own names, bound so far, each
    /// read through the place it is kept in: the value `locals()`
    /// answers with there, and the near dictionary an `eval` reads
    /// there hands over. A member only a conditional's arm may have
    /// bound is left out rather than glanced at, since what it would
    /// show -- nothing written, or what a pass before it left -- is
    /// never the answer the reference gives.
    fn class_names_map(&mut self) -> Form {
        let pairs: Vec<(String, Form)> = {
            let parts = self.parts();
            let uncertain = parts.uncertain.clone();
            parts.ranking.iter().filter(|named| !uncertain.contains(named))
                .filter_map(|named| parts.attributes.iter().position(|kept| kept == named).map(|at| (named.clone(), parts.held[at].clone())))
                .collect()
        };
        let mut value = prim_call(Prim::MakeMap, Vec::new());
        for (key, read_form) in pairs {
            let item = prim_call(Prim::Couple, vec![constant(Value::text(&key)), read_form]);
            value = prim_call(Prim::ExtendLiteral(true, false), vec![value, item]);
        }
        value
    }

    /// Whether the class body about to be read spells `locals` or
    /// `vars` anywhere among its own tokens, a nested block's included,
    /// but never a block that has already closed by the time this
    /// asks. The body's own namespace, where either turns up, is made
    /// before the first statement of the body runs rather than at the
    /// first use found while reading it: a write through `locals()`
    /// as the body's very first statement needs somewhere of its own
    /// to land before the write runs, not made as part of the write.
    /// Answering `true` where neither word is truly read there costs
    /// the body a namespace it never asks for again; answering `false`
    /// where one is would leave that first write nowhere to land, so
    /// this leans toward `true` wherever the words merely turn up.
    fn class_body_names_locals(&self, from: usize, inline: bool) -> bool {
        let mut depth = 0i32;
        for token in &self.tokens[from..] {
            match token.shape {
                Shape::Finish => break,
                Shape::Open if !inline => depth += 1,
                Shape::Close if !inline => {
                    if depth == 0 { break; }
                    depth -= 1;
                }
                Shape::LineEnd if inline && depth == 0 => break,
                Shape::Sign if inline && depth == 0 && self.table.spells("stmt.terminator", &token.lexeme) => break,
                Shape::Bare if token.lexeme == "locals" || token.lexeme == "vars" || token.lexeme == "__classdict__" && self.table.has_any("ext.stmt.class.detail.kind") => return true,
                _ => {}
            }
        }
        false
    }

    /// The address the class body's own `locals()`/`vars()` is kept
    /// in, made the first time either is asked for or written through
    /// there: a plain map seeded with every member the body has bound
    /// so far, the very way `class_names_map` snapshots one, but kept
    /// under an address of its own so a later asking reads the very
    /// dictionary this one made, and every member the body binds
    /// after this point keeps it in step. A second asking, or one
    /// from a nested asking of the same body, answers with the same
    /// place.
    fn class_book(&mut self) -> Form {
        if let Some(book) = self.parts().book.clone() {
            return Form::Read(book);
        }
        let seed = self.class_names_map();
        let seed = if self.table.has_any("ext.stmt.class.builder") { prim_call(Prim::ClassWork(15), vec![seed]) } else { seed };
        let book = self.gensym("locals");
        self.parts().book = Some(book.clone());
        sequence(vec![Form::Write(book.clone(), Box::new(seed)), Form::Read(book)])
    }

    /// After a class member's value has just been written to the
    /// place it is kept in, mirrored into the body's own live
    /// namespace as well, once that namespace exists: so `locals()`
    /// asked for again there sees it, and so does a bare name read of
    /// a name the body has not bound at compile time (`ns['w'] = 5`
    /// followed by `w`, past what the body itself ever wrote). Where
    /// no namespace has been made yet the member's place is the only
    /// place it is kept, exactly as before. The name is remembered as
    /// one the namespace now governs either way, so a later `del` --
    /// of it, or through `locals()` -- leaves the class with no such
    /// member once a namespace does exist, rather than the value its
    /// place still happens to hold.
    fn mirror_member(&mut self, word: &str, place: &Address) -> Option<Form> {
        self.parts().book_tracked.insert(word.to_string());
        let book = self.parts().book.clone()?;
        let key = constant(Value::text(word));
        let value = Form::Read(place.clone());
        let target = self.read_to_write(&book.ident.to_string());
        Some(prim_call(if self.table.has_any("ext.stmt.class.builder") { Prim::ClassWork(19) } else { Prim::Replace }, vec![target, key, value]))
    }

    /// The companion of `mirror_member` for `del name` read in a class
    /// body: the name is taken out of the body's own namespace too,
    /// once that namespace exists and once the name is one the body
    /// has ever mirrored there -- a method or a nested class, kept
    /// only in its place, is left to the place alone, exactly as
    /// before.
    fn mirror_forget(&mut self, word: &str) -> Option<Form> {
        if !self.parts().book_tracked.contains(word) { return None; }
        let book = self.parts().book.clone()?;
        let target = self.read_to_write(&book.ident.to_string());
        let key = constant(Value::text(word));
        let erased = prim_call(if self.table.has_any("ext.stmt.class.builder") { Prim::ClassWork(20) } else { Prim::Erase }, vec![target, key]);
        Some(self.write(&book.ident.to_string(), erased))
    }

    /// A name a statement of a class body is about to write becomes a
    /// member of the class: the body's place for it is settled ahead of
    /// the write, so the write lands there. Anywhere else, and for the
    /// places the builder makes for itself, this does nothing.
    fn claim(&mut self, word: &str) {
        if !self.in_class_body() || word.starts_with('#') { return; }
        let place = self.member_address(word, "attribute");
        self.member_noted(word, place);
    }

    /// The name of an array being written into, addressed as the write
    /// it is: a name first met on the left of a write belongs to the
    /// scope it is written in, and every later reading of it must find
    /// the same cell.
    fn read_to_write(&mut self, name: &str) -> Form {
        let slot = self.address_to_rewrite(name);
        Form::Read(slot)
    }

    /// The binding an array written into stands in. Where names close
    /// over, a write through a place binds nothing: the array is the
    /// one the name already stands for -- the function's own, an
    /// enclosing function's, or the module's -- and never a fresh local
    /// spelt the same, which is the footing `del a[k]` reads it on too.
    fn address_to_rewrite(&mut self, name: &str) -> Address {
        if !self.table.flag("ext.stmt.function.closes_over") {
            return self.address_to_write(name);
        }
        if let Some((depth, names)) = self.class_bindings.last() {
            if *depth == self.layers.len() {
                if let Some(slot) = names.get(name) { return slot.clone(); }
            }
        }
        self.address_to_read(name)
    }

    fn read(&mut self, name: &str) -> Form {
        if self.table.has_any("ext.stmt.class.builder") && self.in_class_body() && !name.starts_with('#') && !self.declared_outside_class(name) {
            if let Some(book) = self.parts().book.clone() {
                let found = self.gensym("class_lookup");
                let read = prim_call(Prim::ClassWork(18), vec![Form::Read(book), constant(Value::text(name))]);
                let present = prim_call(Prim::At, vec![Form::Read(found.clone()), constant(Value::Small(0))]);
                let value = prim_call(Prim::At, vec![Form::Read(found.clone()), constant(Value::Small(1))]);
                let fallback = self.read_fallback(name);
                let selected = self.choose(present, value, fallback);
                return sequence(vec![Form::Write(found, Box::new(read)), selected]);
            }
        }
        if let Some((depth, names)) = self.class_bindings.last() {
            if *depth == self.layers.len() {
                if let Some(slot) = names.get(name).cloned() {
                    let conditional = self.table.flag("ext.syntax.names.shadow_builtins")
                        && self.table.prims.contains_key(name)
                        && self.under_way.last().is_some_and(|body| body.uncertain.iter().any(|word| word == name));
                    if conditional {
                        let fallback = self.read_fallback(name);
                        return self.choose(Form::Missing(slot.clone()), fallback, Form::Read(slot));
                    }
                    return Form::Read(slot);
                }
            }
        }
        // A name the class body has never bound at compile time may
        // still be one `locals()[k] = v` bound there while the body
        // ran, past what the body itself ever wrote: CPython looks
        // such a name up by LOAD_NAME, through the very dictionary
        // `locals()` there answers with, before it falls back to the
        // scope around the class or beyond it. Once the body has made
        // that dictionary, a name it does not know at compile time is
        // asked of it first, and only falls through to the ordinary
        // reading below where the dictionary does not have it either.
        if self.in_class_body() && !name.starts_with('#') {
            if let Some(book) = self.parts().book.clone() {
                let test = prim_call(Prim::Membership, vec![constant(Value::text(name)), Form::Read(book.clone())]);
                let found = prim_call(Prim::At, vec![Form::Read(book), constant(Value::text(name))]);
                let missing = self.read_fallback(name);
                return self.choose(test, found, missing);
            }
        }
        self.read_fallback(name)
    }

    /// What `read` falls back to for a name the class body's own live
    /// namespace does not answer for, or has none of: the ordinary
    /// reading of a name, exactly as before this file's namespace was
    /// given a dictionary of its own.
    fn read_fallback(&mut self, name: &str) -> Form {
        if !self.table.flag("ext.stmt.function.closes_over") && self.outside_lambda.iter().any(|word| word == name) {
            let local = self.layers.iter().rev().find(|scope| scope.holds == Holds::Every)
                .map_or(false, |scope| scope.idents.iter().any(|word| word == name));
            if !local {
                if let Some(said) = self.table.single("ext.op.lambda.enclosing") {
                    return prim_call(Prim::Raise, vec![constant(Value::text(said))]);
                }
            }
        }
        // The word a language uses for the line it is written on stands
        // for that line, which is known while the form is built.
        if self.table.single("ext.system.source.line") == Some(name) {
            return constant(Value::Small((self.look().row).saturating_sub(self.before) as i64));
        }
        // The routine a piece stands in, the class that routine belongs
        // to, and the two written together: each is known while the form
        // is built, so each stands for what it names.
        let routine = self.naming.last().cloned().unwrap_or_default();
        let within = self.within.as_ref().map(|(called, _)| called.clone()).unwrap_or_default();
        for (label, said) in [
            ("ext.system.source.routine", routine.clone()),
            ("ext.system.source.class", within.clone()),
            ("ext.system.source.method", match within.is_empty() {
                true => routine,
                false => format!("{}::{}", within, routine),
            }),
        ] {
            if self.table.single(label) == Some(name) {
                return constant(Value::text(&said));
            }
        }
        if self.in_class_body() && self.table.flag("ext.syntax.names.shadow_builtins") {
            if let Some(operation) = self.table.prims.get(name).copied() {
                let member = self.parts().lexical_members.iter().any(|word| word == name);
                let address = if member && !self.declared_outside_class(name) {
                    self.global_address(name)
                } else { self.address_to_read(name) };
                let global_depth = self.layers.iter().skip(1).filter(|layer| layer.holds != Holds::Nothing).count();
                if address.up == global_depth && !self.named_in_program.iter().any(|word| word == name) {
                    return constant(Value::Intrinsic(operation, Rc::from(name)));
                }
                return Form::Read(address);
            }
        }
        Form::Read(self.address_to_read(name))
    }

    /// A read of a binding about to be overwritten by the very form
    /// that reads it, with nothing else able to see the binding in
    /// between: a comprehension's own gathering place, read once each
    /// step and written back a few steps on. The value moves out of
    /// the binding rather than being cloned out of it.
    fn read_taking(&mut self, name: &str) -> Form {
        Form::Take(self.address_to_read(name))
    }

    fn write(&mut self, name: &str, value: Form) -> Form {
        let slot = self.address_to_write(name);
        // `x = x + k` and `x = x - k` step the binding in place.
        if let Form::Dyad { op: op @ (Prim::Plus | Prim::Minus), a, b, .. } = &value {
            if let (Input::Address(read), Input::Const(Value::Small(k))) = (a, b) {
                if read.ident == slot.ident && read.up == slot.up && read.at == slot.at && read.fallback == slot.fallback {
                    let by = if *op == Prim::Minus { -*k } else { *k };
                    return Form::Bump { slot, by };
                }
            }
        }
        Form::Write(slot, Box::new(value))
    }

    /// The full name of a routine or class: the class around it, then
    /// each routine entered since that class was, with the word for
    /// what it holds locally after each, and its own name last.
    fn full_name_of(&self, name: &str) -> String {
        let local = self.table.single("ext.stmt.class.detail.locals").unwrap_or("");
        let mut path = self.within.as_ref().map(|(c, _)| format!("{c}.")).unwrap_or_default();
        for (outer, afresh) in self.naming.iter().zip(&self.named_afresh).skip(self.named_before) {
            if *afresh { path.clear(); }
            path.push_str(outer); path.push('.'); path.push_str(local); path.push('.');
        }
        if self.global_where_written(name) { path.clear(); }
        path.push_str(name);
        path
    }

    /// Whether the routine being built declared this name global: a
    /// routine written under it then goes by that name alone.
    fn global_where_written(&self, name: &str) -> bool {
        self.naming.len() > self.named_before
            && self.layers.last().map_or(false, |layer| layer.aliases.iter().any(|(said, meant)| said == name && meant == name))
    }

    /// A program value: its body reduced in a scope of its own.
    fn routine(&mut self, name: &str, holds: Holds, catches: Traps, params: Vec<String>, least: usize, body: impl FnOnce(&mut Self) -> Res<Form>) -> Res<Form> {
        let type_params = std::mem::take(&mut self.pending_types);
        let annotator = self.build_annotator()?;
        let began = self.pos;
        let mut start = self.pos;
        while self.tokens.get(start).map_or(false, |t| matches!(t.shape, Shape::Open | Shape::LineEnd) || self.table.spells("block.intro", &t.lexeme)) { start += 1; }
        let expression_body = name == "<generator>" || self.table.strings("ext.stmt.function.anonymous").first().map_or(false, |n| n == name);
        let mut doc = None;
        let mut finish = start;
        while let Some(token) = self.tokens.get(finish).filter(|t| t.shape == Shape::Quote) {
            doc.get_or_insert_with(String::new).push_str(&token.lexeme);
            finish += 1;
        }
        if expression_body || self.tokens.get(finish).map_or(false, |t| t.shape == Shape::Woven) { doc = None; }
        let qualification=self.full_name_of(name);
        let taking = self.taking.take();
        let declared_on = self.declared_at;
        // What the routine around this one carries is set aside while
        // this one is built, so that each keeps only its own.
        let around = std::mem::take(&mut self.carrying);
        // The word before a routine makes it a coroutine; a scope inside
        // it is a coroutine only by a word of its own.
        let coroutine_around = std::mem::replace(&mut self.in_coroutine, std::mem::take(&mut self.coroutine_next));
        let afresh = self.global_where_written(name);
        self.naming.push(name.to_string());
        self.named_afresh.push(afresh);
        // The classes the parameters were written to take, gathered as
        // they were read. A method is handed the thing it is for before
        // them, so the list is brought level with the names.
        let mut formal_kinds = std::mem::take(&mut self.formal_kinds);
        while formal_kinds.len() < params.len() {
            formal_kinds.insert(0, None);
        }
        formal_kinds.truncate(params.len());
        let param_slots = (0..params.len()).collect();
        let permits_async = (holds != Holds::Every || matches!(name, "<gathering>" | "<generator>" | "<genexpr>"))
            && self.layers.last().map_or(false, |scope| scope.permits_async);
        self.layers.push(Layer { async_walk_seen: false, permits_async, gathering_kind: None, expression_targets: Vec::new(), comprehension: name == "<gathering>", borrowed: Vec::new(), class_borrowed: Vec::new(), reaching: Vec::new(), holds, idents: params.clone(), formals: Vec::new(), formal_slots: param_slots, rpn: false, aliases: Vec::new(), encountered: Vec::new() });
        if self.table.flag("ext.stmt.function.closes_over") && !self.survey && holds == Holds::Every {
            if let Some(known) = self.surveyed.get(&began) {
                if !self.table.has_any("ext.builtin.exceptions.syntax") && known.borrowed.iter().any(|word| params.contains(word) && !known.class_borrowed.contains(word)) {
                    return Err(self.table.single("ext.stmt.function.parameters.amiss").unwrap_or_default().into());
                }
                let scope = self.layers.last_mut().unwrap();
                for word in &known.bound {
                    if !scope.idents.contains(word) { scope.idents.push(word.clone()); }
                }
                scope.aliases.clone_from(&known.outermost);
                scope.borrowed.clone_from(&known.borrowed);
                scope.class_borrowed.clone_from(&known.class_borrowed);
            }
        }
        let earlier_gathering = self.gather_names.clone();
        if holds == Holds::Every && self.table.flag("ext.stmt.function.closes_over") {
            let own = &self.layers.last().unwrap().idents;
            self.gather_names.retain(|pair| !own.contains(&pair.0));
        }
        let enclosing_yield = self.generator_seen;
        if holds == Holds::Every { self.generator_seen = false; }
        let outer_declarations = std::mem::take(&mut self.declarations);
        let previous_loops = self.loop_depth;
        let previous_finally = self.finally_nesting;
        if holds == Holds::Every { self.loop_depth = 0; self.finally_nesting = 0; }
        let mut body = body(self)?;
        self.loop_depth = previous_loops;
        self.finally_nesting = previous_finally;
        self.declarations = outer_declarations;
        self.gather_names = earlier_gathering;
        let generator = holds == Holds::Every && self.generator_seen && self.table.flag("ext.stmt.yield.suspends");
        if holds == Holds::Every {
            if self.generator_seen && !generator {
                body = prim_call(Prim::Raise, vec![constant(Value::text(self.table.single("ext.stmt.yield.unrun").unwrap_or_default()))]);
            }
            self.generator_seen = enclosing_yield;
        }
        let scope = self.layers.pop().unwrap();
        if self.survey && holds == Holds::Every {
            let bound = scope.idents.iter().filter(|word| !scope.borrowed.contains(word) && !scope.aliases.iter().any(|pair| pair.0 == **word)).cloned().collect();
            self.surveyed.insert(began, ScopeWords { bound, outermost: scope.aliases.clone(), borrowed: scope.borrowed.clone(), class_borrowed: scope.class_borrowed.clone() });
        }
        if generator && !self.table.flag("ext.stmt.function.closes_over") {
            let names = self.layers.iter().skip(1).filter(|scope| scope.holds == Holds::Every)
                .flat_map(|scope| scope.idents.iter()).filter(|name| !scope.idents.contains(name))
                .map(String::as_str).collect::<Vec<_>>();
            if borrows_enclosing(&body, &names) { body = self.scope_unrun("ext.stmt.yield.unsupported"); }
        }
        self.naming.pop();
        self.named_afresh.pop();
        let carried = std::mem::replace(&mut self.carrying, around);
        self.in_coroutine = coroutine_around;
        let mut locals: Vec<String> = scope.idents.iter().filter(|n| !n.starts_with('#') && !scope.borrowed.contains(n) && !scope.aliases.iter().any(|pair| &pair.0 == *n) && !scope.reaching.iter().any(|a| a.ident.as_ref() == n.as_str())).cloned().collect();
        locals.sort_by_key(|word| params.iter().position(|p| p == word).map_or((3, 0), |slot| {
            let group = taking.as_ref().map_or(0, |r| match r[slot] { 'b' | 'p' => 0, 'n' => 1, _ => 2 });
            (group, slot)
        }));
        let mut flags = 3i64;
        if self.layers.iter().skip(1).any(|s| s.holds == Holds::Every) { flags += 16; }
        for (kind, bit) in [('v', 4), ('k', 8)] {
            if taking.as_ref().map_or(false, |rules| rules.contains(&kind)) { flags |= bit; }
        }
        if scope.permits_async { flags |= 128; } else if generator { flags |= 32; }
        let mut literals = vec![doc.as_ref().map_or(Value::Nil, |s| Value::text(s))];
        let mut referenced = Vec::new();
        let mut suspension = false;
        inspect_form(&body, &locals, &mut literals, &mut referenced, &mut suspension);
        if scope.permits_async && suspension { flags = (flags & !128) | 512; }
        Ok(constant(Value::Routine(Rc::new(Routine { class_namespace: None, annotator, literals, referenced, locals, flags, lineless: false, qualification, doc, generator, local_defaults: Vec::new(), gather_from: None, ident: name.to_string(), least, formals: params, taking, formal_kinds, formal_slots: scope.formal_slots, idents: scope.idents, reaching: scope.reaching, frameless: holds == Holds::Nothing, written_in: self.written_in.clone(), within: self.within.as_ref().map(|(named, _)| Rc::from(named.as_str())), declared_on, type_params, globe: self.globe.clone(), born: self.born.clone(), framed_in: self.framed_in.clone(), traps: catches, carried, body }))))
    }

    fn build_annotator(&mut self) -> Res<Option<Rc<Routine>>> {
        let sites = std::mem::take(&mut self.annotation_sites);
        if sites.len() == 0 { return Ok(None); }
        let resume = self.pos;
        let taking = self.taking.take();
        let kinds = std::mem::take(&mut self.formal_kinds);
        let title = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
        self.pos = sites[0].1;
        let made = self.routine(&title, Holds::Every, Traps::Yields, vec!["format".to_owned()], 1, |b| {
            let mut pairs = Vec::new();
            for (mut word, at) in sites {
                if let Some((owner, _)) = &b.within {
                    let owner = owner.split('.').next_back().unwrap_or(owner).trim_start_matches('_');
                    if word.starts_with("__") && !word.ends_with("__") && !owner.is_empty() {
                        word.insert_str(0, &format!("_{owner}"));
                    }
                }
                b.pos = at;
                let expression = b.expr_at(0, false)?;
                pairs.push(prim_call(Prim::Couple, vec![constant(Value::text(&word)), expression]));
            }
            Ok(prim_call(Prim::MakeMap, pairs))
        });
        self.pos = resume;
        self.taking = taking;
        self.formal_kinds = kinds;
        match made? {
            Form::Const(Value::Routine(r)) => Ok(Some(r)),
            _ => unreachable!(),
        }
    }

    /// A branch arm or a loop body: a program that holds no names.
    fn limb(&mut self, catches: Traps, body: impl FnOnce(&mut Self) -> Res<Form>) -> Res<Form> {
        // An arm that traps nothing is read in place, as part of
        // the program around it, not as a program of its own.
        if catches == Traps::Naught {
            return body(self);
        }
        self.routine("<arm>", Holds::Nothing, catches, Vec::new(), 0, body)
    }

    /// A branch: `if(test, «then», «otherwise»)`, the arms program values
    /// that own no names, so the chosen one runs in the frame around it.
    fn choose(&mut self, test: Form, then: Form, otherwise: Form) -> Form {
        let wrap = |name: &str, body: Form| {
            let program = Routine { class_namespace: None, annotator: None, literals: Vec::new(), referenced: Vec::new(), locals: Vec::new(), flags: 0, lineless: false, qualification: String::new(), doc: None, generator: false, local_defaults: Vec::new(), gather_from: None, ident: name.to_string(), least: 0, formals: Vec::new(), formal_kinds: Vec::new(), taking: None, formal_slots: Vec::new(), idents: Vec::new(), reaching: Vec::new(), frameless: true, written_in: self.written_in.clone(), within: None, declared_on: 0, type_params: Vec::new(), globe: self.globe.clone(), born: self.born.clone(), framed_in: self.framed_in.clone(), traps: Traps::Naught, carried: Vec::new(), body };
            constant(Value::Routine(Rc::new(program)))
        };
        prim_call(Prim::Choose, vec![test, wrap("<then>", then), wrap("<else>", otherwise)])
    }

    /// `loop = « if test { body; step; loop() } »; loop()`. The test,
    /// the body and the step are read inside the programs they run in,
    /// so their names resolve to the right frames.
    fn cycle<T, B, S>(&mut self, test: T, body: B, step: Option<S>) -> Res<Form>
    where
        T: FnOnce(&mut Self) -> Res<Form>,
        B: FnOnce(&mut Self) -> Res<Form>,
        S: FnOnce(&mut Self) -> Res<Form>,
    {
        let test = test(self)?;
        self.loop_depth += 1;
        let body = body(self)?;
        self.loop_depth -= 1;
        let step = match step {
            Some(step) => Some(Box::new(step(self)?)),
            None => None,
        };
        let otherwise = if self.table.flag("ext.stmt.loop.else") {
            self.skip_line_ends();
            if self.key("stmt.else") { self.advance(); Some(Box::new(self.body()?)) } else { None }
        } else { None };
        Ok(Form::Cycle { test: Box::new(test), body: Box::new(body), step, after: false, otherwise })
    }

    /// `loop = « body; if test {} else { loop() } »; loop()`.
    fn until_loop<B, T>(&mut self, body: B, test: T) -> Res<Form>
    where
        B: FnOnce(&mut Self) -> Res<Form>,
        T: FnOnce(&mut Self) -> Res<Form>,
    {
        // Read in source order: the test is written before the body.
        let test = test(self)?;
        let body = body(self)?;
        Ok(Form::Cycle { test: Box::new(test), body: Box::new(body), step: None, after: true, otherwise: None })
    }

    /// The counted loop: bound and variable set, then a loop stepping by one.
    fn count_loop<B>(&mut self, var: &str, start: Form, end: Form, body: B) -> Res<Form>
    where
        B: FnOnce(&mut Self) -> Res<Form>,
    {
        let limit = self.gensym("end");
        let limit_name = limit.ident.to_string();
        let set_limit = Form::Write(limit, Box::new(end));
        let set_var = self.write(var, start);
        let (v1, v2) = (var.to_string(), var.to_string());
        let looped = self.cycle(
            move |r| {
                let v = r.read(&v1);
                let l = r.read(&limit_name);
                Ok(prim_call(Prim::Lt, vec![v, l]))
            },
            body,
            Some(move |r: &mut Self| {
                let v = r.read(&v2);
                let next = prim_call(Prim::Plus, vec![v, constant(Value::Small(1))]);
                Ok(r.write(&v2, next))
            }),
        )?;
        Ok(sequence(vec![set_limit, set_var, looped]))
    }

    // ---------- statements

    fn stmts_until(&mut self, stops: &[String]) -> Res<Form> {
        let mut items = Vec::new();
        self.skip_line_ends();
        while !self.exhausted() && !stops.iter().any(|s| self.lexeme_of(s)) {
            items.push(self.stmt()?);
            self.skip_line_ends();
        }
        Ok(sequence(items))
    }

    fn skip_lead_word(&mut self) {
        if self.on_any("block.intro") {
            self.advance();
        }
    }

    fn body(&mut self) -> Res<Form> {
        if let Some((name, at)) = self.iteration_binding.take() {
            let body_at = self.pos;
            self.pos = at;
            let mut prefix = self.loop_targets(&name)?;
            self.pos = body_at;
            prefix.push(self.body()?);
            return Ok(sequence(prefix));
        }
        // The block of a loop or a watched statement that stands in a
        // class body names members of the class, and is read the way
        // the body around it is read.
        if self.in_class_body() { return self.class_limb(); }
        // A loop with nothing to do may be written with the mark that
        // ends a statement standing where its block would: the mark is
        // the whole body, and nothing runs each pass.
        if self.table.flag("ext.block.lone_statement")
            && self.look().shape == Shape::Sign
            && self.table.separates(&self.look().lexeme)
        {
            self.advance();
            return Ok(constant(Value::Nil));
        }
        let head_mark = self.on_any("block.intro");
        self.skip_lead_word();
        let same_line = head_mark && self.table.blocks == Blocks::Indented && self.table.flag("ext.block.lone_statement")
            && !self.on_stmt_end() && !matches!(self.look().shape, Shape::Open | Shape::Close | Shape::Finish);
        if same_line {
            let mut body = vec![self.stmt()?];
            while self.look().shape == Shape::Sign && self.table.separates(&self.look().lexeme) {
                self.advance();
                if matches!(self.look().shape, Shape::Finish | Shape::Close | Shape::LineEnd) { break; }
                body.push(self.stmt()?);
            }
            return Ok(sequence(body));
        }
        self.skip_line_ends();
        match self.table.blocks {
            Blocks::Indented => {
                if self.look().shape != Shape::Open {
                    return Err(format!("Expected an indented block, got '{}'", self.look().lexeme));
                }
                self.advance();
                let mut items = Vec::new();
                self.skip_line_ends();
                while self.look().shape != Shape::Close && !self.exhausted() {
                    items.push(self.stmt()?);
                    self.skip_line_ends();
                }
                if self.look().shape != Shape::Close {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.advance();
                Ok(sequence(items))
            }
            Blocks::Bracketed => {
                let opens = self.table.strings("block.open");
                let Some(k) = opens.iter().position(|o| self.lexeme_of(o)) else {
                    // A language may open a block with a mark where a
                    // bracket would stand, and close it with a word of
                    // its own. Statements run to whichever such word
                    // comes next: one ending the whole shape is taken
                    // here, one opening another of its arms is left
                    // standing for whoever opened the block.
                    if self.table.single("ext.stmt.block.instead").map_or(false, |m| self.sign(m)) {
                        self.advance();
                        let mut stops = self.table.strings("ext.stmt.block.instead.close").to_vec();
                        stops.extend(self.table.strings("stmt.elif").iter().cloned());
                        stops.extend(self.table.strings("stmt.else").iter().cloned());
                        let body = self.stmts_until(&stops)?;
                        if self.table.strings("ext.stmt.block.instead.close").iter().any(|w| self.lexeme_of(w)) {
                            self.advance();
                        }
                        return Ok(body);
                    }
                    if self.table.flag("ext.block.lone_statement") {
                        return self.stmt();
                    }
                    return Err(format!("Expected '{}' to open a block, got '{}'", opens[0], self.look().lexeme));
                };
                self.advance();
                let close = self.table.strings("block.close")[k].clone();
                let body = self.stmts_until(std::slice::from_ref(&close))?;
                self.need_lexeme(&close)?;
                Ok(body)
            }
            _ => {
                let closers = self.table.strings("block.close").to_vec();
                let body = self.stmts_until(&closers)?;
                self.need_closer()?;
                Ok(body)
            }
        }
    }

    /// A block in a scope of its own, as an arm program parsed in place.
    fn body_limb(&mut self, catches: Traps) -> Res<Form> {
        self.limb(catches, |r| r.body())
    }

    fn type_scope_fault(&self) -> Option<String> {
        if !self.table.has_any("ext.builtin.exceptions.syntax") { return None; }
        let word = |at: usize| self.tokens.get(at).map_or("", |token| token.lexeme.as_str());
        let head = word(self.pos);
        let named = match head {
            "async" if word(self.pos + 1) == "def" => self.pos + 2,
            "type" | "class" | "def" => self.pos + 1,
            _ => return None,
        };
        if self.tokens.get(named)?.shape != Shape::Bare { return None; }
        let mut cursor = named + 1;
        let has_types = word(cursor) == "[";
        if has_types {
            cursor += 1;
            if word(cursor) == "]" { return Some(String::from("SyntaxError: Type parameter list cannot be empty")); }
            loop {
                let category = if word(cursor) == "**" { cursor += 1; "ParamSpec" }
                    else if word(cursor) == "*" { cursor += 1; "TypeVarTuple" }
                    else { "TypeVar" };
                if self.tokens.get(cursor)?.shape != Shape::Bare { break; }
                cursor += 1;
                let relation = word(cursor);
                if relation == ":" || relation == "=" {
                    cursor += 1;
                    let begin = cursor;
                    let mut nested = Vec::new();
                    while let Some(token) = self.tokens.get(cursor) {
                        let text = token.lexeme.as_str();
                        if nested.is_empty() && (text == "," || text == "]" || matches!(token.shape, Shape::LineEnd | Shape::Finish)) { break; }
                        match text {
                            "(" | "[" | "{" => nested.push(text),
                            ")" | "]" | "}" => { nested.pop(); },
                            _ => {}
                        }
                        cursor += 1;
                    }
                    let constrained = relation == ":" && word(begin) == "(" && {
                        let mut level = 0usize;
                        let mut found = false;
                        for place in begin..cursor {
                            match word(place) {
                                "(" | "[" | "{" => level += 1,
                                ")" | "]" | "}" => { level = level.saturating_sub(1); if level == 0 { break; } },
                                "," if level == 1 => found = true,
                                _ => {}
                            }
                        }
                        found
                    };
                    let place = if relation == "=" { format!("{category} default") }
                        else if constrained { String::from("TypeVar constraint") }
                        else { String::from("TypeVar bound") };
                    if let Some(invalid) = (begin..cursor).find_map(|index| match word(index) {
                        ":=" => Some("named"), "yield" => Some("yield"), "await" => Some("await"), _ => None,
                    }) { return Some(format!("SyntaxError: {invalid} expression cannot be used within a {place}")); }
                }
                match word(cursor) {
                    "]" => { cursor += 1; break; },
                    "," => cursor += 1,
                    _ => break,
                }
            }
        }
        if head == "class" && has_types && word(cursor) == "(" {
            let mut openings = 0usize;
            for token in self.tokens.iter().skip(cursor) {
                if let Some(kind) = match token.lexeme.as_str() { ":=" => Some("named"), "yield" => Some("yield"), "await" => Some("await"), _ => None } {
                    return Some(format!("SyntaxError: {kind} expression cannot be used within the definition of a generic"));
                }
                match token.lexeme.as_str() {
                    "(" => openings += 1,
                    ")" => { openings = openings.saturating_sub(1); if openings == 0 { break; } },
                    _ => {}
                }
                if matches!(token.shape, Shape::LineEnd | Shape::Finish) { break; }
            }
        }
        if head == "type" && word(cursor) == "=" {
            let mut inside = 0usize;
            for token in self.tokens.iter().skip(cursor + 1) {
                if inside == 0 && (matches!(token.shape, Shape::LineEnd | Shape::Close | Shape::Finish) || token.lexeme == ";") { break; }
                let bad = match token.lexeme.as_str() { ":=" => "named", "yield" => "yield", "await" => "await", _ => "" };
                if !bad.is_empty() { return Some(format!("SyntaxError: {bad} expression cannot be used within a type alias")); }
                if ["(", "[", "{"].contains(&token.lexeme.as_str()) { inside += 1; }
                else if [")", "]", "}"].contains(&token.lexeme.as_str()) { inside = inside.saturating_sub(1); }
            }
        }
        None
    }

    fn stmt(&mut self) -> Res<Form> {
        if let Some(problem) = self.type_scope_fault() { return Err(problem); }
        let opened = self.pos;
        self.warnings_in_statement(opened);
        let made = self.stmt_of_line()?;
        self.past_stmt(opened)?;
        Ok(made)
    }

    /// The token past the statement opened at `opened`: its line end,
    /// or the edge of a block, met outside every bracket.
    fn statement_limit(&self, opened: usize) -> usize {
        let mut limit = opened;
        let mut nesting = 0usize;
        while let Some(token) = self.tokens.get(limit) {
            if nesting == 0 && matches!(token.shape, Shape::LineEnd | Shape::Finish | Shape::Open | Shape::Close) { break; }
            if token.shape == Shape::Sign {
                match token.lexeme.as_str() {
                    "(" | "[" | "{" => nesting += 1,
                    ")" | "]" | "}" => nesting = nesting.saturating_sub(1),
                    _ => {}
                }
            }
            limit += 1;
        }
        limit
    }

    /// The closing bracket paired with the one opened at `at`, before `limit`.
    fn pair_close(&self, at: usize, limit: usize) -> Option<usize> {
        let mut nesting = 0usize;
        for here in at..limit {
            let token = &self.tokens[here];
            if token.shape != Shape::Sign { continue; }
            match token.lexeme.as_str() {
                "(" | "[" | "{" => nesting += 1,
                ")" | "]" | "}" => { nesting -= 1; if nesting == 0 { return Some(here); } }
                _ => {}
            }
        }
        None
    }

    /// The opening bracket paired with the one closed at `at`, at or after `floor`.
    fn pair_open(&self, at: usize, floor: usize) -> Option<usize> {
        let mut nesting = 0usize;
        let mut here = at;
        loop {
            let token = &self.tokens[here];
            if token.shape == Shape::Sign {
                match token.lexeme.as_str() {
                    ")" | "]" | "}" => nesting += 1,
                    "(" | "[" | "{" => { nesting -= 1; if nesting == 0 { return Some(here); } }
                    _ => {}
                }
            }
            if here == floor { return None; }
            here -= 1;
        }
    }

    /// Whether one of the signs stands unbracketed between the pair.
    fn pair_holds(&self, open: usize, close: usize, signs: &[&str]) -> bool {
        let mut nesting = 0usize;
        for here in open + 1..close {
            let token = &self.tokens[here];
            if token.shape != Shape::Sign { continue; }
            match token.lexeme.as_str() {
                "(" | "[" | "{" => nesting += 1,
                ")" | "]" | "}" => nesting = nesting.saturating_sub(1),
                other if nesting == 0 && signs.contains(&other) => return true,
                _ => {}
            }
        }
        false
    }

    /// A literal beginning at `at` -- a number, text, bytes, or a display
    /// of a tuple, list, dictionary or set -- as the reference names its
    /// kind, with the token past it. A bracketed expression is none.
    fn literal_at(&self, at: usize, limit: usize) -> Option<(&'static str, usize)> {
        let token = self.tokens.get(at)?;
        if at >= limit { return None; }
        match token.shape {
            Shape::Numeral => {
                // The reference folds numbers joined by arithmetic signs
                // into one number before it looks; the chain is read as
                // that one number, real where any part or a division is.
                let mut kind = Self::numeral_kind(&token.lexeme);
                let mut past = at + 1;
                while past + 1 < limit && Self::joins_operands(&self.tokens[past]) && self.tokens[past].lexeme != "." && self.tokens[past + 1].shape == Shape::Numeral {
                    let next = Self::numeral_kind(&self.tokens[past + 1].lexeme);
                    kind = if kind == "complex" || next == "complex" { "complex" }
                        else if kind == "float" || next == "float" || self.tokens[past].lexeme == "/" { "float" }
                        else { "int" };
                    past += 2;
                }
                Some((kind, past))
            }
            Shape::Quote => {
                let mut past = at + 1;
                while past < limit && self.tokens[past].shape == Shape::Quote { past += 1; }
                Some(("str", past))
            }
            Shape::ByteQuote => Some(("bytes", at + 1)),
            Shape::Woven => {
                // A woven string reads as text wherever it stands; a
                // field in it opens a string of its own, so the ends
                // and openings of one are counted through.
                let mut depth = 1usize;
                let mut past = at + 1;
                while past < limit && depth > 0 {
                    match self.tokens[past].shape {
                        Shape::Woven => depth += 1,
                        Shape::WovenEnd => depth -= 1,
                        _ => {}
                    }
                    past += 1;
                }
                if depth > 0 { return None; }
                let woven_template = token.lexeme.chars().next().map_or(false, |c| self.table.spells("ext.lexical.string.prefix.template", &c.to_string()));
                Some((if woven_template { "string.templatelib.Template" } else { "str" }, past))
            }

            Shape::Sign => {
                let opener = token.lexeme.as_str();
                if !["(", "[", "{"].contains(&opener) { return None; }
                let close = self.pair_close(at, limit)?;
                let kind = if opener == "[" { "list" }
                    else if opener == "{" { if close == at + 1 || self.pair_holds(at, close, &[":"]) { "dict" } else { "set" } }
                    else if opener == "(" && self.tokens.get(at + 1).map_or(false, |t| t.shape == Shape::Bare && self.table.spells("ext.op.lambda", &t.lexeme)) { "function" }
                    else if self.pair_holds(at, close, &[","]) { "tuple" }
                    else if close == at + 2 { self.literal_at(at + 1, close)?.0 }
                    else if opener == "(" && self.pair_gathers(at, close) { "generator" }
                    else { return None };
                Some((kind, close + 1))
            }
            _ => None,
        }
    }

    /// Whether the paired stretch gathers: the walk word standing at
    /// the pair's own depth.
    fn pair_gathers(&self, open: usize, close: usize) -> bool {
        let walks = self.table.strings("ext.op.comprehension.for");
        if walks.is_empty() { return false; }
        let mut nesting = 0usize;
        for here in open + 1..close {
            let token = &self.tokens[here];
            match token.shape {
                Shape::Sign => match token.lexeme.as_str() {
                    "(" | "[" | "{" => nesting += 1,
                    ")" | "]" | "}" => nesting = nesting.saturating_sub(1),
                    _ => {}
                },
                Shape::Bare if nesting == 0 && walks.iter().any(|word| word == &token.lexeme) => return true,
                _ => {}
            }
        }
        false
    }

    /// The reference's name for a number's kind, from its spelling.
    fn numeral_kind(lexeme: &str) -> &'static str {
        let spelling = lexeme.to_ascii_lowercase();
        let prefixed = ["0x", "0o", "0b"].iter().any(|prefix| spelling.starts_with(prefix));
        if spelling.ends_with('j') { "complex" }
        else if !prefixed && (spelling.contains('.') || spelling.contains('e')) { "float" }
        else { "int" }
    }

    /// The kind of a literal whose last token stands just before `past`,
    /// where the operand ending there is that literal alone: not a call
    /// or a subscript of it, and not a wider expression it ends.
    fn literal_ending(&self, past: usize, opened: usize) -> Option<&'static str> {
        if past <= opened { return None; }
        let last = past - 1;
        let token = &self.tokens[last];
        let closes = token.shape == Shape::Sign && [")", "]", "}"].contains(&token.lexeme.as_str());
        let begin = if closes {
            let open = self.pair_open(last, opened)?;
            if open > opened {
                let ahead = &self.tokens[open - 1];
                let names = ahead.shape == Shape::Bare && !self.table.keywords.contains(&ahead.lexeme);
                let closed = ahead.shape == Shape::Sign && [")", "]", "}"].contains(&ahead.lexeme.as_str());
                if names || closed { return None; }
            }
            open
        } else {
            let mut begin = last;
            if matches!(token.shape, Shape::WovenEnd | Shape::Field) {
                // Fields open strings of their own within the string:
                // walk back over each to the beginning of the whole.
                let mut depth = 0usize;
                while begin > opened {
                    match self.tokens[begin].shape {
                        Shape::WovenEnd => depth += 1,
                        Shape::Woven => {
                            if depth == 0 { break; }
                            depth -= 1;
                            if depth == 0 { break; }
                        }
                        _ => {}
                    }
                    begin -= 1;
                }
            }
            while token.shape == Shape::Quote && begin > opened && self.tokens[begin - 1].shape == Shape::Quote { begin -= 1; }
            // Numbers reached through arithmetic signs from a number are
            // the one folded number, back to the first of them.
            while token.shape == Shape::Numeral && begin > opened + 1 && Self::joins_operands(&self.tokens[begin - 1]) && self.tokens[begin - 1].lexeme != "."
                && self.tokens[begin - 2].shape == Shape::Numeral { begin -= 2; }
            begin
        };
        let (kind, ends) = self.literal_at(begin, past)?;
        if ends != past { return None; }
        if begin > opened && Self::joins_operands(&self.tokens[begin - 1]) { return None; }
        Some(kind)
    }

    /// Whether the token is an operator sign that makes a literal part
    /// of a wider expression rather than an operand on its own.
    fn joins_operands(token: &Token) -> bool {
        token.shape == Shape::Sign && ["+", "-", "*", "/", "//", "%", "**", "@", "&", "|", "^", "<<", ">>", "."].contains(&token.lexeme.as_str())
    }

    /// The kind of the one expression between the index marks, as the
    /// reference names it: a comma parting them makes a tuple, a cut
    /// mark a slice that indexes fine, and an integer indexes fine. A
    /// name or a larger expression says nothing.
    fn index_kind(&self, open: usize, close: usize) -> Option<&'static str> {
        if self.pair_holds(open, close, &[","]) { return Some("tuple"); }
        if self.pair_holds(open, close, &[":"]) { return None; }
        if open + 1 >= close { return None; }
        if open + 2 == close {
            let only = &self.tokens[open + 1];
            if only.shape == Shape::Bare {
                if self.table.spells("literal.null", &only.lexeme) { return Some("NoneType"); }
                if self.table.spells("literal.true", &only.lexeme) || self.table.spells("literal.false", &only.lexeme) { return Some("bool"); }
            }
            if only.shape == Shape::Sign && self.table.spells("ext.literal.ellipsis", &only.lexeme) { return Some("ellipsis"); }
        }
        let (kind, ends) = self.literal_at(open + 1, close)?;
        if ends != close { return None; }
        Some(kind)
    }

    /// The kind a named constant is of, where one stands just before
    /// the call's opening mark at `here`: a call upon it warns as a
    /// call upon that kind.
    fn constant_kind_called(&self, here: usize) -> Option<&'static str> {
        let before = &self.tokens[here - 1];
        if before.shape == Shape::Sign && self.table.spells("ext.literal.ellipsis", &before.lexeme) { return Some("ellipsis"); }
        if before.shape != Shape::Bare { return None; }
        if self.table.spells("literal.null", &before.lexeme) { return Some("NoneType"); }
        if self.table.spells("literal.true", &before.lexeme) || self.table.spells("literal.false", &before.lexeme) { return Some("bool"); }
        None
    }

    /// What the reference warns of in the statement opened at `opened`,
    /// noted for whoever compiled the text: an assertion of a
    /// parenthesised tuple, an identity test against a literal, and a
    /// call made upon a literal. Each place is noted once.
    fn warnings_in_statement(&mut self, opened: usize) {
        let table = self.table;
        if !table.has_any("ext.builtin.exceptions.syntax") || !table.has_any("ext.system.syntax_warnings") { return; }
        let limit = self.statement_limit(opened);
        let mut noted: Vec<(String, u32, usize)> = Vec::new();
        let first = &self.tokens[opened];
        if first.shape == Shape::Bare && table.spells("ext.stmt.assert", &first.lexeme) && opened + 1 < limit && self.tokens[opened + 1].shape == Shape::Sign && self.tokens[opened + 1].lexeme == "(" {
            if let Some(close) = self.pair_close(opened + 1, limit) {
                let next = self.tokens.get(close + 1);
                let ends = close + 1 >= limit || next.map_or(true, |t| t.shape == Shape::Sign && (t.lexeme == "," || table.separates(&t.lexeme)));
                if ends && self.pair_holds(opened + 1, close, &[","]) {
                    let bracket = &self.tokens[opened + 1];
                    noted.push(("assertion is always true, perhaps remove parentheses?".to_owned(), bracket.row, bracket.column));
                }
            }
        }
        for here in opened..limit {
            let token = &self.tokens[here];
            if token.shape == Shape::Numeral {
                if let Some(next) = self.tokens.get(here + 1) {
                    if here + 1 < limit && next.shape == Shape::Bare && next.row == token.row
                        && next.column == token.column + token.lexeme.chars().count()
                        && ["and", "or", "in", "not", "if", "else", "for", "is"].contains(&next.lexeme.as_str()) {
                        let label = if token.lexeme.starts_with("0x") { "hexadecimal" }
                            else if token.lexeme.starts_with("0o") { "octal" }
                            else if token.lexeme.starts_with("0b") { "binary" } else { "decimal" };
                        noted.push((format!("invalid {label} literal"), token.row, next.column));
                    }
                }
            }
            if token.shape == Shape::Bare && table.spells("ext.op.identical", &token.lexeme) {
                let negated = here + 1 < limit && self.tokens[here + 1].shape == Shape::Bare && table.spells("ext.op.identical.negated", &self.tokens[here + 1].lexeme);
                let mut right = here + 1 + usize::from(negated);
                // The reference folds a sign before a number into the number.
                let signed = right + 1 < limit && self.tokens[right].shape == Shape::Sign && ["-", "+"].contains(&self.tokens[right].lexeme.as_str()) && self.tokens[right + 1].shape == Shape::Numeral;
                if signed { right += 1; }
                let after = self.literal_at(right, limit).filter(|(_, past)| *past >= limit || !Self::joins_operands(&self.tokens[*past])).map(|(kind, _)| kind);
                if let Some(kind) = after.or_else(|| self.literal_ending(here, opened)) {
                    let (spoken, meant) = if negated { ("is not", "!=") } else { ("is", "==") };
                    noted.push((format!("\"{spoken}\" with '{kind}' literal. Did you mean \"{meant}\"?"), token.row, token.column));
                }
            }
            if here > opened && token.shape == Shape::Sign && token.lexeme == "(" {
                // A function made on the spot is called on purpose: the
                // reference warns of no call upon one, only of indexing.
                let kind = self.literal_ending(here, opened).or_else(|| self.constant_kind_called(here));
                if let Some(kind) = kind.filter(|k| *k != "function") {
                    noted.push((format!("'{kind}' object is not callable; perhaps you missed a comma?"), token.row, token.column));
                }
            }
            // A set, a generator, a function or a template indexed right
            // after it stands warns too, as does indexing a number or a
            // named constant; text, tuples, lists and maps index fine.
            if here > opened && token.shape == Shape::Sign && token.lexeme == "[" {
                let kind = self.literal_ending(here, opened).or_else(|| self.constant_kind_called(here));
                if let Some(kind) = kind.filter(|k| !matches!(*k, "str" | "bytes" | "tuple" | "list" | "dict")) {
                    noted.push((format!("'{kind}' object is not subscriptable; perhaps you missed a comma?"), token.row, token.column));
                } else if let Some(value_kind) = kind.filter(|k| matches!(*k, "str" | "bytes" | "tuple" | "list")) {
                    // Text, tuples and lists index fine with an integer
                    // or a slice; indexing one with anything else warns.
                    if let Some(index) = self.pair_close(here, limit).and_then(|close| self.index_kind(here, close)).filter(|k| !matches!(*k, "int" | "bool")) {
                        noted.push((format!("{value_kind} indices must be integers or slices, not {index}; perhaps you missed a comma?"), token.row, token.column));
                    }
                }
            }
        }
        for warning in noted {
            if !self.warnings.contains(&warning) { self.warnings.push(warning); }
        }
    }

    /// After a statement of a line-ended language must come the line's
    /// end, a separator, or the edge of the block. What else stands
    /// there is refused; a lone `print` or `exec` with an expression
    /// after it is the statement the language once spelled that way.
    /// The reference's warning that a return, break or continue sits
    /// inside a `finally` block, noted once where the word stands for
    /// whoever compiled the text.
    fn note_finally_word(&mut self, word: &str) {
        if self.finally_nesting == 0 { return; }
        let at = self.look();
        let noted = (format!("'{word}' in a 'finally' block"), at.row, at.column);
        if !self.warnings.contains(&noted) { self.warnings.push(noted); }
    }

    fn past_stmt(&mut self, opened: usize) -> Res<()> {
        let table = self.table;
        if !table.has_any("ext.stmt.legacy_call") || !table.has_any("ext.builtin.exceptions.syntax") { return Ok(()); }
        let shape = self.look().shape;
        if self.on_stmt_end() || matches!(shape, Shape::Finish | Shape::Close | Shape::Open | Shape::Lead) { return Ok(()); }
        // Compound statements step over the line ends past their block
        // to look for another arm; a token on a later line than the
        // statement's last token therefore opens the next statement.
        if self.pos <= opened { return Ok(()); }
        let previous = &self.tokens[self.pos - 1];
        // A block's close stands on the line after the block, so a
        // statement ending on a token that shapes the block is over.
        if matches!(previous.shape, Shape::LineEnd | Shape::Close | Shape::Open | Shape::Lead | Shape::Finish) { return Ok(()); }
        let ended_on = if previous.end_row != 0 { previous.end_row } else { previous.row };
        if self.look().row > ended_on { return Ok(()); }
        let first = self.tokens[opened].clone();
        let lone_word = opened + 1 == self.pos && first.shape == Shape::Bare;
        if lone_word && table.spells("ext.stmt.legacy_call", &first.lexeme) && !self.on_any("ext.stmt.yield") {
            let here = self.pos;
            let expression = self.comma_value().is_ok();
            self.pos = here;
            if expression {
                let word = first.lexeme;
                return Err(format!("SyntaxError: Missing parentheses in call to '{word}'. Did you mean {word}(...)?"));
            }
        }
        if previous.shape == Shape::Quote && self.look().shape == Shape::Bare
            && self.tokens[self.pos..].iter().take_while(|word| word.row == previous.row)
                .any(|word| word.shape == Shape::Quote) {
            return Err(String::from("SyntaxError: invalid syntax. Is this intended to be part of the string?"));
        }
        Err("SyntaxError: invalid syntax".to_owned())
    }

    fn stmt_of_line(&mut self) -> Res<Form> {
        // A language that says where a complaint happened wants each
        // statement to carry the line it was written on.
        // Only the program's own lines are carried: what stands ahead of
        // it is the library, and a complaint from within that names the
        // line of the program that was running, as PHP names it.
        if self.look().row > self.before {
            self.past_library = true;
        }
        if self.tells_place && self.look().row > self.before {
            let row = self.look().row - self.before;
            let made = self.plain_or_kind()?;
            return Ok(Form::OnLine(row, Box::new(made)));
        }
        self.plain_or_kind()
    }

    fn scope_unrun(&self, label: &str) -> Form {
        prim_call(Prim::Raise, vec![constant(Value::text(self.table.single(label).unwrap_or("")))])
    }

    /// The body's names are read within a layer that cannot escape it.
    fn class_scope(&mut self) -> Res<Form> {
        self.advance();
        let name = self.need_word("as the class name")?;
        if self.table.has_any("ext.builtin.exceptions.syntax") && name == "__debug__" { return Err("SyntaxError: cannot assign to __debug__".to_owned()); }
        if self.on_any("ext.stmt.type_params.open") { self.class_type_parameters()?; }
        if self.table.flag("ext.stmt.function.closes_over") { self.address_to_write(&name); }
        if self.on_any("ext.stmt.class.bases.open") {
            self.advance();
            let _bases = self.arguments_of(&name, "ext.stmt.class.bases.close", "syntax.call.separator")?;
        }
        let _members = self.routine(&name, Holds::Every, Traps::Naught, Vec::new(), 0, |b| b.body())?;
        Ok(self.scope_unrun("ext.stmt.class.unready"))
    }

    /// Read a value whose following commas may gather a tuple.
    fn comma_value(&mut self) -> Res<Form> { self.comma_expression(true) }

    fn comma_expression(&mut self, writes: bool) -> Res<Form> {
        let (first, spreads) = self.comma_part(writes)?;
        if !self.on_any("ext.op.tuple") {
            return if spreads { Err(self.table.single("ext.stmt.unpack.amiss").unwrap_or("Invalid tuple").to_string()) }
                else { Ok(first) };
        }
        let mut whole = if spreads { first } else { prim_call(Prim::MakeArray, vec![first]) };
        loop {
            self.advance();
            let ended = self.on_stmt_end() || self.exhausted() || self.look().shape == Shape::Close
                || self.on_any("block.intro") || self.on_any("syntax.group.close")
                || self.on_any("syntax.array.close") || self.on_any("syntax.map.close");
            if ended { break; }
            // A yield expression joined by a comma it did not open is
            // no element: the reference wants it parenthesised there.
            if self.table.has_any("ext.builtin.exceptions.syntax") && self.key("ext.stmt.yield") {
                return Err(String::from("SyntaxError: invalid syntax"));
            }
            let (part, spread) = self.comma_part(writes)?;
            let portion = if spread { part } else { prim_call(Prim::MakeArray, vec![part]) };
            whole = prim_call(Prim::TupleJoined, vec![whole, portion]);
            if !self.on_any("ext.op.tuple") { break; }
        }
        Ok(if self.table.has_any("ext.builtin.tuple") { prim_call(Prim::Tupling, vec![whole]) } else { whole })
    }

    fn comma_tail(&mut self, first: Form) -> Res<Form> {
        if !self.on_any("ext.op.tuple") { return Ok(first); }
        let mut value = prim_call(Prim::MakeArray, vec![first]);
        loop {
            self.advance();
            if self.on_stmt_end() || self.on_assign() || self.on_any("syntax.group.close")
                || self.on_any("block.intro") || matches!(self.look().shape, Shape::Finish | Shape::Close) { break; }
            if self.table.has_any("ext.builtin.exceptions.syntax") && self.key("ext.stmt.yield") {
                return Err(String::from("SyntaxError: invalid syntax"));
            }
            let (part, expanded) = self.comma_part(true)?;
            let segment = if expanded { part } else { prim_call(Prim::MakeArray, vec![part]) };
            value = prim_call(Prim::TupleJoined, vec![value, segment]);
            if !self.on_any("ext.op.tuple") { break; }
        }
        Ok(if self.table.has_any("ext.builtin.tuple") { prim_call(Prim::Tupling, vec![value]) } else { value })
    }

    fn plain_or_kind(&mut self) -> Res<Form> {
        if self.table.has_any("ext.builtin.exceptions.syntax") {
            let correction = match self.look().lexeme.as_str() {
                "fur" => Some("for"), "whille" => Some("while"), "iff" => Some("if"),
                "elseif" => Some("elif"), "tyo" => Some("try"), "classe" => Some("class"),
                "impor" => Some("import"), "form" | "frum" => Some("from"),
                "defn" => Some("def"), "returm" => Some("return"), "lamda" => Some("lambda"),
                "yeld" => Some("yield"), "globel" => Some("global"),
                "asynch" => Some("async"), "awaid" => Some("await"),
                "raisee" => Some("raise"), _ => None,
            };
            if let Some(target) = correction {
                let following = self.glance(1);
                if following.shape == Shape::Bare || self.look().lexeme == "tyo" && following.lexeme == ":" && self.glance(2).shape == Shape::LineEnd {
                    return Err(format!("SyntaxError: invalid syntax. Did you mean '{target}'?"));
                }
            }
            if self.look().lexeme == "elso" && self.glance(1).lexeme == ":"
                && self.glance(2).shape == Shape::LineEnd {
                return Err(String::from("SyntaxError: invalid syntax. Did you mean 'else'?"));
            }
            if self.look().lexeme == "case" && self.glance(1).lexeme != ":" {
                let mut nested = 0usize;
                let arm = self.tokens.iter().skip(self.pos + 1).take_while(|token| !matches!(token.shape, Shape::LineEnd | Shape::Close | Shape::Finish)).any(|token| {
                    if ["(", "[", "{"].contains(&token.lexeme.as_str()) { nested += 1; }
                    else if [")", "]", "}"].contains(&token.lexeme.as_str()) { nested = nested.saturating_sub(1); }
                    token.lexeme == ":" && nested == 0
                });
                if arm { return Err("SyntaxError: case statement must be inside match statement".to_owned()); }
            }
            if self.look().lexeme == "lazy" && ["import", "from"].contains(&self.glance(1).lexeme.as_str()) {
                let from = self.glance(1).lexeme == "from";
                let disallowed = if self.in_class_body() { Some("inside classes") }
                    else if !self.naming.is_empty() { Some("inside functions") }
                    else if self.syntax_try_nesting != 0 { Some("inside try/except blocks") }
                    else { None };
                let words = self.tokens[self.pos..].iter().take_while(|token|
                    !matches!(token.shape, Shape::LineEnd | Shape::Close | Shape::Finish) && token.lexeme != ";").collect::<Vec<_>>();
                let terminal = words.last().copied().unwrap_or(self.look());
                if let Some(place) = disallowed {
                    self.range_end = Some((if terminal.end_column == 0 { terminal.column + terminal.lexeme.chars().count() } else { terminal.end_column }, terminal.end_row.max(terminal.row)));
                    let kind = if from { "from ... import" } else { "import" };
                    return Err(format!("SyntaxError: lazy {kind} not allowed {place}"));
                }
                if from && words.windows(2).any(|pair| pair[0].lexeme == "import" && pair[1].lexeme == "*") {
                    self.range_end = Some((if terminal.end_column == 0 { terminal.column + terminal.lexeme.chars().count() } else { terminal.end_column }, terminal.end_row.max(terminal.row)));
                    return Err("SyntaxError: lazy from ... import * is not allowed".to_owned());
                }
            }
            let outer = self.key("ext.stmt.global");
            if (outer || self.key("ext.stmt.nonlocal")) && !self.in_class_body() {
                let origin = self.pos;
                let scope = self.layers.last().unwrap();
                let parameters: Vec<String> = scope.formal_slots.iter().filter_map(|at| scope.idents.get(*at)).cloned().collect();
                let encountered = scope.encountered.clone();
                let mut cursor = origin + 1;
                while let Some(token) = self.tokens.get(cursor) {
                    if matches!(token.shape, Shape::LineEnd | Shape::Close | Shape::Finish) || token.lexeme == ";" { break; }
                    if token.shape == Shape::Bare {
                        let named = token.lexeme.clone();
                        let previous = self.declarations.iter().find(|(_, word, was_outer)| word == &named && *was_outer != outer).map(|(at, _, _)| *at);
                        // A declaration said again is no fault; said
                        // once, what the name already did in this scope
                        // words it.
                        let declared_same = self.declarations.iter().any(|(_, word, was_outer)| word == &named && *was_outer == outer);
                        let met_kind = if declared_same { None }
                            else { [MET_READ, MET_ANNOTATED, MET_WRITTEN].into_iter().find(|kind| encountered.iter().any(|(n, k)| n == &named && k == kind)) };
                        let before = if outer { "global" } else { "nonlocal" };
                        let complaint = if parameters.contains(&named) { Some(format!("name '{named}' is parameter and {}", if outer { "global" } else { "nonlocal" })) }
                            else if previous.is_some() { Some(format!("name '{named}' is nonlocal and global")) }
                            else { met_kind.map(|kind| match kind {
                                MET_READ => format!("name '{named}' is used prior to {before} declaration"),
                                MET_ANNOTATED => format!("annotated name '{named}' can't be {before}"),
                                _ => format!("name '{named}' is assigned to before {before} declaration"),
                            }) };
                        if let Some(complaint) = complaint {
                            self.pos = previous.unwrap_or(origin);
                            let mut last = self.look();
                            for t in self.tokens.iter().skip(self.pos) {
                                if matches!(t.shape, Shape::LineEnd | Shape::Close | Shape::Finish) || t.lexeme == ";" { break; }
                                last = t;
                            }
                            self.range_end = Some((last.column + last.lexeme.chars().count(), last.row));
                            return Err(format!("SyntaxError: {complaint}"));
                        }
                        self.declarations.push((origin, named, outer));
                    }
                    cursor += 1;
                }
            }
            if self.key("stmt.if") || self.key("stmt.while") {
                let mut balance = 0;
                let mut assigned = false;
                let mut member = false;
                for at in self.pos + 1..self.tokens.len() {
                    let t = &self.tokens[at];
                    if matches!(t.shape, Shape::Finish | Shape::LineEnd) { break; }
                    if t.shape != Shape::Sign { continue; }
                    match t.lexeme.as_str() {
                        "(" | "[" | "{" => balance += 1,
                        ")" | "]" | "}" => balance -= 1,
                        "=" if balance == 0 => {
                            assigned = true;
                            member = at > 1 && self.tokens[at - 2].lexeme == "." && self.tokens[at - 1].shape == Shape::Bare;
                        },
                        ":" if balance == 0 => {
                            if assigned {
                                let end = &self.tokens[at - 1];
                                self.range_end = Some((end.column + end.lexeme.chars().count(), end.row));
                                self.advance();
                                let message = if member { "SyntaxError: cannot assign to attribute here. Maybe you meant '==' instead of '='?" }
                                    else { "SyntaxError: invalid syntax. Maybe you meant '==' or ':=' instead of '='?" };
                                return Err(message.to_owned());
                            }
                            break;
                        }
                        _ => ()
                    }
                }
            }
            let text = if self.key("stmt.return") && (self.naming.is_empty() || self.in_class_body()) {
                Some("'return' outside function")
            } else if self.loop_depth == 0 || self.in_class_body() {
                if self.key("stmt.break") { Some("'break' outside loop") }
                else if self.key("stmt.continue") { Some("'continue' not properly in loop") } else { None }
            } else { None };
            if let Some(text) = text {
                let mut end = self.look().column;
                for token in &self.tokens[self.pos..] {
                    if matches!(token.shape, Shape::LineEnd | Shape::Close | Shape::Finish) || token.lexeme == ";" { break; }
                    end = if token.end_column == 0 { token.column + token.lexeme.chars().count() } else { token.end_column };
                }
                self.range_end = Some((end, self.look().row));
                return Err(format!("SyntaxError: {text}"));
            }
        }
        let next = self.glance(1);
        let ends = matches!(next.shape, Shape::Finish | Shape::Close | Shape::LineEnd)
            || self.table.spells("stmt.terminator", &next.lexeme);
        if ends && self.on_any("ext.literal.ellipsis") {
            self.advance();
            return Ok(constant(Value::Nil));
        }
        if self.look().shape == Shape::Bare || self.on_any("ext.stmt.decorator") {
            if self.key("stmt.let") {
                return self.bind();
            }
            // The deferring word ahead of an import is stepped over; the
            // import behind it is read and answered as any other.
            if self.key("ext.stmt.import.lazy") && self.glance(1).shape == Shape::Bare
                && (self.table.spells("ext.stmt.import", &self.glance(1).lexeme) || self.table.spells("ext.stmt.import.from", &self.glance(1).lexeme)) {
                self.advance();
                self.in_lazy_from = true;
                let read = self.stmt();
                self.in_lazy_from = false;
                return read;
            }
            if self.key("ext.stmt.async") {
                let word = self.advance().lexeme;
                if self.key("stmt.for") || self.key("ext.stmt.with") {
                    if !self.layers.last().unwrap().permits_async || self.in_class_body() {
                        let head = if self.key("stmt.for") { "async for" } else { "async with" };
                        return Err(format!("SyntaxError: '{head}' outside async function"));
                    }
                    if self.layers.len() == 1 { self.top_coroutine = true; }
                }
                if !self.key("stmt.function") && !self.key("stmt.for") && !self.key("ext.stmt.with") {
                    return Err(format!("Expected a function, for loop or with block after '{}', got '{}'", word, self.look().lexeme));
                }
                // A marked function runs as functions do and is a
                // coroutine; a marked loop or context block is read as
                // asynchronous, and may stand only within a coroutine.
                self.coroutine_next = self.key("stmt.function");
                self.asynchronous = self.key("stmt.for") || self.key("ext.stmt.with");
                if self.asynchronous && !self.in_coroutine && !(self.layers.len() == 1 && self.layers[0].permits_async) && self.table.has_any("ext.builtin.exceptions.syntax") {
                    return Err(format!("SyntaxError: '{} {}' outside async function", word, self.look().lexeme));
                }
                return self.stmt();
            }
            if self.key("ext.stmt.with") { return self.with_block(); }
            if self.key("ext.stmt.del") {
                self.advance();
                return self.forget_list(false, true, None);
            }
            if self.key("ext.stmt.nonlocal") {
                if self.table.flag("ext.stmt.function.closes_over") && !self.layers.iter().skip(1).any(|layer| layer.holds == Holds::Every) && self.class_bindings.is_empty() {
                    if self.table.has_any("ext.builtin.exceptions.syntax") {
                        let mut last = self.look();
                        for word in &self.tokens[self.pos..] {
                            if matches!(word.shape, Shape::LineEnd | Shape::Finish | Shape::Close) || word.lexeme == ";" { break; }
                            last = word;
                        }
                        self.range_end = Some((last.column + last.lexeme.chars().count(), last.row));
                    }
                    return Err(self.table.single("ext.stmt.nonlocal.module").unwrap_or_default().into());
                }
                let in_class = self.class_bindings.last().map_or(false, |(level, _)| *level == self.layers.len());
                self.advance();
                loop {
                    let word = self.need_word("after the nonlocal keyword")?;
                    // A class body pushes no layer of its own, so the
                    // layer around it is the very one a name declared
                    // `nonlocal` there means to reach: the declaration
                    // is kept where any other's is, and a write of the
                    // name within the body goes there rather than to a
                    // member, exactly as a name declared `global` does.
                    if let Some(layer) = self.layers.iter_mut().rev().find(|l| l.holds == Holds::Every) {
                        layer.borrowed.push(word.clone());
                        if in_class { layer.class_borrowed.push(word.clone()); }
                    }
                    // Outside a class body the walk this looks for a
                    // binding with must pass over the very function the
                    // declaration stands in, since a name meant there
                    // is no `nonlocal` of that function's own. A class
                    // body is no function of its own to pass over this
                    // way: the walk includes the one around it, since
                    // that is the very function the declaration means.
                    if !self.survey && self.table.flag("ext.stmt.function.closes_over") && self.lexical_address(&word, !in_class).is_none() {
                        let pieces = self.table.strings("ext.stmt.nonlocal.amiss");
                        return Err(format!("{}{}{}", pieces.first().map_or("", String::as_str), word, pieces.get(1).map_or("", String::as_str)));
                    }
                    if !self.on_any("syntax.call.separator") { break; }
                    self.advance();
                }
                if self.table.flag("ext.stmt.function.closes_over") { return Ok(constant(Value::Nil)); }
                return Ok(prim_call(Prim::Raise, vec![constant(Value::text(self.table.single("ext.stmt.nonlocal.unrun").unwrap_or_default()))]));
            }
            if self.key("stmt.if") {
                return self.if_stmt();
            }
            // `do body while (c);`: the body runs before its test is
            // asked, so it runs at least once. Said as a loop that stops
            // once the test does not hold, tested after the body.
            if self.key("ext.stmt.do") {
                self.advance();
                let body = self.body()?;
                if !self.key("stmt.while") {
                    return Err(format!("Expected '{}' after the body, got '{}'", self.table.strings("stmt.while").first().map_or("while", |w| w.as_str()), self.look().lexeme));
                }
                self.advance();
                let open = self.table.single("syntax.group.open").ok_or_else(|| "A do loop needs syntax.group".to_string())?;
                self.need_sign(open, "after while")?;
                let test = self.expr(0)?;
                self.need_sign(self.table.single("syntax.group.close").unwrap(), "after the condition")?;
                let stops = prim_call(Prim::Invert, vec![test]);
                return Ok(Form::Cycle { test: Box::new(stops), body: Box::new(body), step: None, after: true, otherwise: None });
            }
            if self.key("stmt.while") {
                self.advance();
                let test_at = self.pos;
                return self.cycle(
                    move |r| {
                        r.pos = test_at;
                        r.expr(0)
                    },
                    |r| r.body(),
                    None::<fn(&mut Self) -> Res<Form>>,
                );
            }
            if self.key("stmt.until") {
                self.advance();
                let test_at = self.pos;
                // The test is written before the body; find where it ends,
                // then read the body and, inside the loop, the test.
                let _ = self.expr(0)?;
                let body_at = self.pos;
                return self.until_loop(
                    move |r| {
                        r.pos = body_at;
                        r.body()
                    },
                    move |r| {
                        let after = r.pos;
                        r.pos = test_at;
                        let test = r.expr(0)?;
                        r.pos = after;
                        Ok(test)
                    },
                );
            }
            if self.key("stmt.for") {
                return self.for_stmt();
            }
            if self.key("ext.stmt.type_alias") && self.glance(1).shape == Shape::Bare {
                let earlier = std::mem::replace(&mut self.forbids_await, true);
                self.advance();
                let alias = self.need_word("as the type alias")?;
                if self.table.has_any("ext.builtin.exceptions.syntax") && alias == "__debug__" {
                    return Err(String::from("SyntaxError: cannot assign to __debug__"));
                }
                self.type_names()?;
                self.pending_types.clear();
                self.need_assign("after the type alias")?;
                // What the alias stands for is worked out only when it
                // is asked for, since names in it need not mean anything
                // yet: the name is bound to a routine answering it.
                let made = self.routine(&alias, Holds::Every, Traps::Yields, Vec::new(), 0, |b| {
                    let worth = if b.table.has_any("ext.op.tuple") { b.comma_value()? } else { b.expr(0)? };
                    Ok(prim_call(Prim::Yield, vec![worth]))
                });
                self.forbids_await = earlier;
                let thunk = made?;
                return Ok(self.write(&alias, thunk));
            }
            if self.key("stmt.return") {
                self.note_finally_word("return");
                self.advance();
                // A routine giving back a cell answers with the cell of
                // whatever it names, so a name tied to the answer and
                // the one within stand for one cell. Where what it names
                // has no cell, the value itself is the answer, as such a
                // language does rather than stopping.
                let by_cell = self.giving_cells.last().copied().unwrap_or(false);
                let value = if self.on_stmt_end() || self.exhausted() || self.on_any("block.close") {
                    Vec::new()
                } else if by_cell {
                    vec![self.a_shared_cell(&self.table.strings("ext.op.reference.unshared.given").to_vec(), true, None)?]
                } else {
                    vec![if self.table.has_any("ext.op.tuple") { self.comma_value()? } else { self.expr(0)? }]
                };
                return Ok(prim_call(Prim::Yield, value));
            }
            if self.key("stmt.break") {
                self.note_finally_word("break");
                self.advance();
                let levels = self.loop_levels()?;
                return Ok(prim_call(Prim::Leave, levels));
            }
            if self.key("stmt.continue") {
                self.note_finally_word("continue");
                self.advance();
                let levels = self.loop_levels()?;
                return Ok(prim_call(Prim::Resume, levels));
            }
            if self.key("stmt.function") {
                self.advance();
                let gives_cell = self.skip_reference();
                let name = self.need_word("after the function keyword")?;
                self.giving_cells.push(gives_cell);
                let built = self.func(name, true);
                self.giving_cells.pop();
                return built;
            }
            if self.key("stmt.pass") {
                self.advance();
                return Ok(constant(Value::Nil));
            }
            if self.key("ext.stmt.class.trait") {
                return self.bag_decl();
            }
            if self.key("ext.stmt.class") || self.key("ext.stmt.class.interface") {
                if self.table.has_any("ext.builtin.exceptions.syntax") && self.glance(1).lexeme == "__debug__" {
                    return Err("SyntaxError: cannot assign to __debug__".to_owned());
                }
                if !self.table.flag("ext.stmt.class.this.explicit") && self.table.has_any("ext.stmt.class.bases.open") { return self.class_scope(); }
                return self.class_decl();
            }
            // A class may be marked before it is named: `abstract class C`.
            if self.key("ext.stmt.class.modifier") && self.glance(1).shape == Shape::Bare {
                let next = self.glance(1).lexeme.clone();
                if self.table.spells("ext.stmt.class", &next) || self.table.spells("ext.stmt.class.interface", &next) {
                    self.advance();
                    return self.class_decl();
                }
            }
            if self.key("ext.stmt.try") {
                return self.attempt_stmt();
            }
            if self.key("ext.stmt.throw") {
                self.advance();
                if self.table.has_any("ext.builtin.exceptions.syntax") && self.key("ext.stmt.throw.from") {
                    return Err("SyntaxError: did you forget an expression between 'raise' and 'from'?".to_owned());
                }
                if self.table.single("ext.stmt.throw.from").is_some()
                    && (matches!(self.look().shape, Shape::LineEnd | Shape::Close | Shape::Finish) || self.on_any("stmt.terminator"))
                {
                    return Ok(Form::Again);
                }
                let mut values = vec![self.expr(0)?];
                if self.key("ext.stmt.throw.from") {
                    self.advance();
                    if self.table.has_any("ext.builtin.exceptions.syntax") && (self.on_stmt_end() || matches!(self.look().shape, Shape::Close | Shape::Finish)) {
                        return Err("SyntaxError: did you forget an expression after 'from'?".to_owned());
                    }
                    let cause = self.expr(0)?;
                    if self.table.has_any("ext.builtin.exceptions") { values.push(cause); }
                }
                return Ok(prim_call(Prim::Hurl, values));
            }
            if self.key("ext.stmt.assert") {
                self.advance();
                if self.table.has_any("ext.builtin.exceptions.syntax") {
                    if !self.divided_at(self.pos, self.tokens.len(), "ext.op.assign.expression").is_empty() {
                        return Err(String::from("SyntaxError: cannot use named expression without parentheses here"));
                    }
                    let equals = self.divided_at(self.pos, self.tokens.len(), "stmt.assign");
                    if let Some(&stop) = equals.first() {
                        let words = &self.tokens[self.pos..stop];
                        let named = if words.first().is_some_and(|t| t.lexeme == "(") && words.iter().any(|t| t.lexeme == "yield") {
                            Some("yield expression")
                        } else if words.len() > 2 && words[0].shape == Shape::Bare && words[1].lexeme == "["
                            && self.pair_close(self.pos + 1, stop) == Some(stop - 1) { Some("subscript") }
                        else if stop > self.pos && self.tokens[stop - 1].shape == Shape::Bare { Some("name") }
                        else { None };
                        if let Some(name) = named { return Err(format!("SyntaxError: cannot assign to {name} here. Maybe you meant '==' instead of '='?")); }
                    }
                }
                let condition = Box::new(self.expr(0)?);
                let message = if self.on_any("syntax.call.separator") {
                    self.advance();
                    self.expr(0)?
                // No message given is told apart from empty text.
                } else { constant(Value::Unset) };
                return Ok(Form::Assert { condition, message: Box::new(message) });
            }
            if self.key("stmt.foreach") {
                return self.foreach_stmt();
            }
            if self.key("ext.stmt.for.c") {
                return self.three_part_for();
            }
            if self.begins_match() { return self.match_stmt(); }
            if self.key("ext.stmt.switch") {
                return self.switch_stmt();
            }
            if self.key("ext.stmt.import.from") || self.key("ext.stmt.import") {
                return self.import_bindings();
            }
            if self.key("ext.stmt.global") {
                return self.global_names();
            }
            if self.on_any("ext.stmt.decorator") {
                return self.decorate();
            }
            if self.key("ext.stmt.static") {
                return self.static_names();
            }
            if self.key("ext.stmt.const") {
                self.advance();
                let name = self.need_word("after the constant keyword")?;
                self.need_assign("after the constant name")?;
                let value = self.expr(0)?;
                let slot = self.global_address(&name);
                return Ok(Form::Write(slot, Box::new(value)));
            }
        }
        if self.table.blocks == Blocks::Bracketed && self.on_any("block.open") {
            // A bare block: a program owning what it first binds, run at once.
            let program = self.routine("<block>", Holds::Fresh, Traps::Naught, Vec::new(), 0, |r| r.body())?;
            return Ok(invoke(program, Vec::new()));
        }
        self.plain_stmt()
    }

    /// Read the words of a module path as words, never as calls.
    fn module_path(&mut self, dotted: bool) -> Res<String> {
        let mut path = Vec::new();
        loop {
            let said = self.original_words[self.pos].lexeme.clone();
            self.need_word("among imported names")?;
            let pieces = match (dotted, self.table.single("op.pipe")) {
                (true, Some(mark)) => said.split(mark).collect::<Vec<_>>(),
                _ => vec![said.as_str()],
            };
            if pieces.iter().any(|part| !self.table.name_like(part) || self.table.keywords.contains(*part)) {
                return Err(format!("Expected identifier among imported names, got '{}'", said));
            }
            path.extend(pieces.iter().map(|s| s.to_string()));
            if !dotted || !self.on_any("op.pipe") {
                return Ok(path.join(self.table.single("op.pipe").unwrap_or(".")));
            }
            self.advance();
        }
    }

    fn future_allowed(&self, stop: usize) -> bool {
        let words: Vec<&Token> = self.tokens[..stop].iter().filter(|t| t.row > self.before).collect();
        let mut count = 0;
        for statement in words.split(|t| t.shape == Shape::LineEnd || t.lexeme == ";") {
            if statement.is_empty() { continue; }
            let quoted = count == 0 && statement.iter().all(|t| t.shape == Shape::Quote);
            let importing = statement.get(0).map_or(false, |t| t.lexeme == "from")
                && statement.get(1).map_or(false, |t| t.lexeme == "__future__")
                && statement.get(2).map_or(false, |t| t.lexeme == "import");
            if !quoted && !importing { return false; }
            count += 1;
        }
        true
    }

    /// Paths remain whole, to be fetched when this statement is reached.
    fn import_bindings(&mut self) -> Res<Form> {
        let statement_at = self.pos;
        let taking_names = self.key("ext.stmt.import.from");
        self.advance();
        let mut path = String::new();
        if taking_names {
            let start = self.pos;
            let mut last_dot = None;
            while self.on_any("op.pipe") || self.on_any("ext.op.index.slice.ellipsis") {
                last_dot = Some(self.look().clone());
                path.push_str(&self.look().lexeme);
                self.advance();
            }
            // A relative import written `from . lazy import`, the lazy
            // word after the dots and set off by a space or a break, is
            // the older order; the reference warns and reads it as the
            // same import without the word.
            if !self.in_lazy_from && self.pos != start && self.look().lexeme == "lazy"
                && self.glance(1).shape == Shape::Bare && self.table.spells("ext.stmt.import", &self.glance(1).lexeme)
                && last_dot.map_or(false, |dot: Token| {
                    let flush = dot.column + dot.lexeme.chars().count();
                    self.look().row != dot.row || self.look().column != flush
                }) {
                let at = self.look().clone();
                let noticed = (format!("did you mean 'lazy from {path} import'?"), at.row, at.column);
                if !self.warnings.contains(&noticed) { self.warnings.push(noticed); }
                self.advance();
            }
            if self.pos == start || !self.key("ext.stmt.import") {
                path.push_str(&self.module_path(true)?);
            }
            if self.table.has_any("ext.builtin.exceptions.syntax") && self.look().lexeme == "lazy" {
                return Err("SyntaxError: use 'lazy from ... ' instead of 'from ... lazy import'".to_owned());
            }
            if self.key("ext.stmt.import") {
                self.advance();
            } else {
                return Err(format!("Expected '{}' following the module path, got '{}'", self.table.single("ext.stmt.import").unwrap_or_default(), self.look().lexeme));
            }
        }
        let future = taking_names && path == "__future__" && self.table.has_any("ext.builtin.exceptions.syntax");
        if future && (!self.future_allowed(statement_at) || self.in_class_body() || self.layers.iter().skip(1).any(|l| l.holds == Holds::Every)) {
            let mut last = &self.tokens[statement_at];
            for token in self.tokens.iter().skip(statement_at) {
                if matches!(token.shape, Shape::Finish | Shape::Close | Shape::LineEnd) || token.lexeme == ";" { break; }
                last = token;
            }
            self.range_end = Some((last.column + last.lexeme.chars().count(), last.row));
            self.pos = statement_at;
            return Err(String::from("SyntaxError: from __future__ imports must occur at the beginning of the file"));
        }
        if self.table.has_any("ext.builtin.exceptions.syntax")
            && matches!(self.look().shape, Shape::LineEnd | Shape::Close | Shape::Finish) {
            return Err(String::from("SyntaxError: Expected one or more names after 'import'"));
        }
        let enclosed = taking_names && self.on_any("syntax.group.open");
        if enclosed {
            self.advance();
        }
        let mut writes = Vec::new();
        if taking_names && !enclosed && self.on_any("op.mul") {
            if self.table.has_any("ext.builtin.exceptions.syntax") && (self.in_class_body() || self.layers.iter().skip(1).any(|scope| scope.holds == Holds::Every)) {
                return Err(String::from("SyntaxError: import * only allowed at module level"));
            }
            self.advance();
            if self.table.flag("ext.stmt.import.value") {
                writes.push(prim_call(Prim::SpreadModule, vec![prim_call(Prim::BringModule, vec![constant(Value::text(&path)), constant(Value::Nil), constant(Value::Flag(false))])]));
            }
        } else {
            loop {
                let named_at = self.pos;
                let original = self.module_path(!taking_names)?;
                let alias = self.key("ext.stmt.import.as");
                let local = match alias {
                    false if original.contains('.') => original.split('.').next().unwrap().to_owned(),
                    false => self.tokens[named_at].lexeme.clone(),
                    true => {
                        self.advance();
                        if self.table.has_any("ext.builtin.exceptions.syntax") {
                            let kind = match self.look().lexeme.as_str() { "(" => Some("tuple"), "[" => Some("list"), _ => None };
                            if let Some(name) = kind { return Err(format!("SyntaxError: cannot use {name} as import target")); }
                            if matches!(self.look().shape, Shape::Numeral | Shape::Quote | Shape::ByteQuote) {
                                return Err(String::from("SyntaxError: cannot use literal as import target"));
                            }
                        }
                        let binding = self.look().lexeme.clone();
                        self.module_path(false)?;
                        if self.table.has_any("ext.builtin.exceptions.syntax") {
                            let error = if self.on_any("op.pipe") { Some("attribute") }
                                else if self.on_any("syntax.call.open") { Some("function call") }
                                else if self.on_any("syntax.array.open") { Some("subscript") }
                                else { None };
                            if let Some(name) = error { return Err(format!("SyntaxError: cannot use {name} as import target")); }
                        }
                        binding
                    }
                };
                if self.table.has_any("ext.builtin.exceptions.syntax") && local == "__debug__" { return Err("SyntaxError: cannot assign to __debug__".to_owned()); }
                if future && !matches!(original.as_str(), "nested_scopes" | "generators" | "division" | "absolute_import" | "with_statement" | "print_function" | "unicode_literals" | "barry_as_FLUFL" | "generator_stop" | "annotations") {
                    let ending = &self.tokens[self.pos - 1];
                    self.range_end = Some((ending.column + ending.lexeme.chars().count(), ending.row));
                    self.pos = named_at;
                    let complaint = match original.as_str() { "braces" => "not a chance".to_owned(), name => format!("future feature {name} is not defined") };
                    return Err(format!("SyntaxError: {complaint}"));
                }
                let worth = if self.table.flag("ext.stmt.import.value") {
                    prim_call(Prim::BringModule, vec![constant(Value::text(if taking_names { &path } else { &original })), constant(if taking_names { Value::text(&original) } else { Value::Nil }), constant(Value::Flag(!taking_names && !alias))])
                } else { constant(Value::Nil) };
                self.claim(&local);
                self.importing = true;
                writes.push(self.write(&local, worth));
                self.importing = false;
                if !self.on_any("syntax.call.separator") {
                    break;
                }
                self.advance();
                if taking_names && !enclosed && self.table.has_any("ext.builtin.exceptions.syntax")
                    && matches!(self.look().shape, Shape::LineEnd | Shape::Close | Shape::Finish) {
                    return Err(String::from("SyntaxError: trailing comma not allowed without surrounding parentheses"));
                }
                if enclosed && self.on_any("syntax.group.close") {
                    break;
                }
            }
        }
        if !taking_names && self.table.has_any("ext.builtin.exceptions.syntax") && self.key("ext.stmt.import.from") {
            return Err("SyntaxError: Did you mean to use 'from ... import ...' instead?".to_owned());
        }
        if enclosed {
            let closing = self.table.single("syntax.group.close").unwrap_or_default().to_string();
            self.need_sign(&closing, "after the import list")?;
        }
        match self.look().shape {
            Shape::Close | Shape::Finish => {},
            _ if self.on_stmt_end() => {},
            _ => return Err(format!("Unexpected token '{}' following imported names", self.look().lexeme)),
        }
        Ok(sequence(writes))
    }

    /// A decorator stands where a `namedexpr_test` may. A reserved word
    /// can lead one only where the table also reads it as the opening of
    /// a value; the rest, `pass` among them, open a statement instead.
    fn adornment_head_ok(&self) -> bool {
        let table = self.table;
        let head = &self.look().lexeme;
        if !table.keywords.contains(head) || table.monadic.contains_key(head) { return true; }
        ["ext.op.lambda", "ext.op.await", "literal.true", "literal.false", "literal.null", "ext.literal.ellipsis"]
            .iter().any(|label| table.spells(label, head))
    }

    /// The writes before the definition gather its decorators. Those
    /// after it rebind the name, taking the gathered values backwards.
    fn decorate(&mut self) -> Res<Form> {
        let mut forms = Vec::new();
        let mut decorators = Vec::new();
        loop {
            let mark = self.advance().lexeme;
            if !self.adornment_head_ok() {
                return Err(self.table.single("ext.stmt.decorator.amiss").unwrap_or_default().to_string());
            }
            let value = self.expr_at(0, false)?;
            let mut cell = self.gensym("adornment");
            // A complaint names the mark the reader wrote, though the
            // value lives in a cell the program cannot name.
            cell.ident = Rc::from(mark.as_str());
            forms.push(Form::Write(cell.clone(), Box::new(value)));
            decorators.push(cell);
            match self.look().shape {
                Shape::LineEnd => {
                    self.advance();
                    while self.look().shape == Shape::LineEnd {
                        self.advance();
                    }
                }
                _ => return Err(self.table.single("ext.stmt.decorator.amiss").unwrap_or_default().to_string()),
            }
            if !self.on_any("ext.stmt.decorator") {
                break;
            }
        }
        if self.key("ext.stmt.async") { self.advance(); self.coroutine_next = true; }
        let named;
        let class_binding = self.key("ext.stmt.class") && self.table.flag("ext.stmt.class.this.explicit");
        if self.key("ext.stmt.class") && self.table.flag("ext.stmt.class.this.explicit") {
            named = self.glance(1).lexeme.clone();
            let (definition, cannot) = self.class_with_receiver()?;
            if cannot { return Ok(self.class_not_ready()); }
            forms.push(definition);
        } else {
            if !self.key("stmt.function") {
                return Err(self.table.single("ext.stmt.decorator.amiss").unwrap_or_default().to_string());
            }
            self.advance();
            let shared = self.skip_reference();
            named = self.need_word("after the function keyword")?;
            self.giving_cells.push(shared);
            let definition = self.func(named.clone(), true);
            self.giving_cells.pop();
            forms.push(definition?);
        }
        let binding = match self.table.flag("ext.stmt.function.outermost") && !class_binding {
            true => self.global_address(&named),
            false => self.address_to_write(&named),
        };
        while let Some(saved) = decorators.pop() {
            let answer = invoke(Form::Read(saved), vec![Form::Read(binding.clone())]);
            forms.push(Form::Write(binding.clone(), Box::new(answer)));
        }
        Ok(sequence(forms))
    }

    /// The items are found and bound in order, then the body is run.
    fn with_block(&mut self) -> Res<Form> {
        let table = self.table;
        let asynchronous = std::mem::take(&mut self.asynchronous);
        self.advance();
        if table.has_any("ext.builtin.exceptions.syntax") {
            let mut opened = Vec::new();
            for part in self.tokens.iter().skip(self.pos) {
                if opened.is_empty() && part.lexeme == ":" { break; }
                if opened.is_empty() && matches!(part.shape, Shape::LineEnd | Shape::Finish) {
                    return Err(String::from("SyntaxError: expected ':'"));
                }
                match part.lexeme.as_str() {
                    "(" | "[" | "{" => opened.push(part.lexeme.as_str()),
                    ")" | "]" | "}" => { opened.pop(); },
                    _ => (),
                }
            }
        }
        let (open, close) = (table.single("syntax.group.open").unwrap(), table.single("syntax.group.close").unwrap());
        let mut enclosed = false;
        if self.sign(open) {
            let mut nesting = 1;
            let mut distance = 1;
            while self.glance(distance).shape != Shape::Finish {
                let token = self.glance(distance);
                if token.shape == Shape::Sign {
                    if token.lexeme == open { nesting += 1; }
                    if token.lexeme == close { nesting -= 1; }
                }
                if nesting == 0 {
                    enclosed = table.spells("block.intro", &self.glance(distance + 1).lexeme);
                    break;
                }
                distance += 1;
            }
        }
        if enclosed { self.advance(); }
        let mut steps = Vec::new();
        let mut contexts = Vec::new();
        loop {
            let begin = self.pos;
            let expression = self.expr(0)?;
            let bounds = self.span_since(begin);
            let mut value = self.located(bounds, expression);
            if table.strings("ext.stmt.class.special").get(33).is_some() {
                let manager = self.gensym("manager");
                let manager_read = Form::Read(manager.clone());
                steps.push(Form::Write(manager.clone(), Box::new(value)));
                let watched = if asynchronous {
                    // An asynchronous manager is entered by its own word;
                    // its leaving method, bound now, stands in the watched
                    // place and is called with the outcome at the end.
                    let leaving = self.gensym("leaving");
                    steps.push(Form::Write(leaving.clone(), Box::new(self.located(bounds, prim_call(Prim::AsyncContext(true), vec![manager_read.clone()])))));
                    value = prim_call(Prim::AsyncContext(false), vec![manager_read]);
                    leaving
                } else {
                    value = prim_call(Prim::StartContext, vec![manager_read]);
                    manager
                };
                value = self.located(bounds, value);
                let entered = self.gensym("entered");
                steps.push(Form::Write(entered.clone(), Box::new(value)));
                value = Form::Read(entered);
                contexts.push((steps.len(), watched, bounds));
            }
            if self.key("ext.stmt.with.as") {
                self.advance();
                let place = self.gensym("with");
                let name = place.ident.to_string();
                steps.push(Form::Write(place, Box::new(value)));
                let start = self.pos;
                let mut boundary = start;
                let mut closing = Vec::new();
                for token in &self.tokens[start..] {
                    let spelling = token.lexeme.as_str();
                    if closing.is_empty() && (["block.intro", "syntax.call.separator", "syntax.group.close"].iter()
                        .any(|label| table.spells(label, spelling))) { break; }
                    match spelling {
                        "(" => closing.push(")"), "[" => closing.push("]"), "{" => closing.push("}"),
                        _ if closing.last().copied() == Some(spelling) => { closing.pop(); }
                        _ => {}
                    }
                    boundary += 1;
                }
                if table.has_any("ext.builtin.exceptions.syntax") && self.has_added_target(start..boundary) {
                    return Err(String::from("SyntaxError: cannot assign to expression"));
                }
                steps.push(self.distribute(start..boundary, &name).map_err(|e| self.loop_target_error(start..boundary, e))?);
                self.pos = boundary;
            } else { steps.push(value); }
            if !self.on_any("syntax.call.separator") { break; }
            self.advance();
            if !enclosed && table.has_any("ext.builtin.exceptions.syntax") && self.on_any("block.intro") {
                return Err("SyntaxError: the last 'with' item has a trailing comma".to_owned());
            }
            if enclosed && self.sign(close) { break; }
        }
        if enclosed { self.need_sign(close, "after the with items")?; }
        if table.has_any("ext.builtin.exceptions.syntax") {
            if self.look().lexeme == "ad" && self.glance(1).shape == Shape::Bare {
                return Err(String::from("SyntaxError: invalid syntax. Did you mean 'and'?"));
            }
            if !self.on_any("block.intro") { return Err(String::from("SyntaxError: expected ':'")); }
        }
        steps.push(self.body()?);
        for (from, manager, bounds) in contexts.into_iter().rev() {
            let enclosed = sequence(steps.split_off(from));
            steps.push(self.located(bounds, Form::Attempt { context: Some(manager), async_context: asynchronous, body: Box::new(enclosed), clauses: Vec::new(), last: None, otherwise: None }));
        }
        Ok(sequence(steps))
    }

    fn targets_ahead(&self, bracketed: bool) -> (usize, bool, bool) {
        let mut offset = usize::from(bracketed);
        let mut nesting = 0;
        let mut width = 0;
        let mut new_item = true;
        let mut separated = false;
        let mut spread = false;
        loop {
            let token = self.glance(offset);
            if token.shape == Shape::Finish { break; }
            let opens = token.shape == Shape::Sign && (self.table.spells("syntax.group.open", &token.lexeme) || self.table.spells("syntax.array.open", &token.lexeme));
            let closes = token.shape == Shape::Sign && (self.table.spells("syntax.group.close", &token.lexeme) || self.table.spells("syntax.array.close", &token.lexeme));
            if nesting == 0 {
                if (bracketed && closes) || (!bracketed && (self.table.spells("stmt.for.in", &token.lexeme) || self.table.spells("stmt.assign", &token.lexeme))) { break; }
                if self.table.spells("syntax.call.separator", &token.lexeme) {
                    separated = true;
                    new_item = true;
                    offset += 1;
                    continue;
                }
                if new_item { width += 1; new_item = false; }
                spread |= self.table.spells("op.mul", &token.lexeme);
            }
            nesting += if opens { 1 } else if closes { -1 } else { 0 };
            offset += 1;
        }
        (width, separated, spread)
    }

    fn require_items(&mut self, source: &str, width: usize, spread: bool) -> Form {
        let refusal = prim_call(Prim::Raise, vec![constant(Value::text(self.table.single("ext.stmt.binding.unrun").unwrap_or_default()))]);
        if spread { return refusal; }
        if self.table.flag("ext.stmt.yield.suspends") {
            let value = self.read(source);
            return self.write(source, prim_call(Prim::BindingWidth(width), vec![value]));
        }
        let value = self.read(source);
        let size = prim_call(Prim::Length, vec![value]);
        let right = prim_call(Prim::Eq, vec![size, constant(Value::Small(width as i64))]);
        self.choose(right, constant(Value::Nil), refusal)
    }

    fn loop_targets(&mut self, source: &str) -> Res<Vec<Form>> {
        let (width, separated, spread) = self.targets_ahead(false);
        if !separated { return self.with_target(source); }
        let mut steps = vec![self.require_items(source, width, spread)];
        for index in 0..width {
            let value = self.read(source);
            let part = prim_call(Prim::At, vec![value, constant(Value::Small(index as i64))]);
            let stored = self.gensym("item");
            let name = stored.ident.to_string();
            steps.push(Form::Write(stored, Box::new(part)));
            steps.extend(self.with_target(&name)?);
            if self.on_any("syntax.call.separator") { self.advance(); }
        }
        Ok(steps)
    }

    fn with_target(&mut self, source: &str) -> Res<Vec<Form>> {
        let table = self.table;
        if table.has_any("ext.builtin.exceptions.syntax") && matches!(self.look().shape, Shape::Numeral | Shape::Quote | Shape::ByteQuote) {
            return Err(String::from("SyntaxError: cannot assign to literal"));
        }
        if table.has_any("ext.builtin.exceptions.syntax") && ["literal.null", "literal.true", "literal.false"].iter().any(|label| self.key(label)) {
            let literal = &self.look().lexeme;
            return Err(format!("SyntaxError: cannot assign to {literal}"));
        }
        if self.on_any("op.mul") {
            self.advance();
            let mut steps = vec![prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.stmt.binding.unrun").unwrap_or_default()))])];
            steps.extend(self.with_target(source)?);
            return Ok(steps);
        }
        let end = if self.on_any("syntax.group.open") { table.single("syntax.group.close") }
            else if self.on_any("syntax.array.open") { table.single("syntax.array.close") } else { None };
        if let Some(end) = end {
            let (width, separated, spread) = self.targets_ahead(true);
            let grouped = width == 1 && !separated && self.on_any("syntax.group.open");
            self.advance();
            if grouped {
                let forms = self.with_target(source)?;
                self.need_sign(end, "after the binding target")?;
                return Ok(forms);
            }
            let mut forms = vec![self.require_items(source, width, spread)];
            let mut position = 0;
            while !self.sign(end) {
                let part = self.gensym("part");
                let part_name = part.ident.to_string();
                let value = self.read(source);
                let item = prim_call(Prim::At, vec![value, constant(Value::Small(position))]);
                forms.push(Form::Write(part, Box::new(item)));
                forms.extend(self.with_target(&part_name)?);
                position += 1;
                if !self.on_any("syntax.call.separator") { break; }
                self.advance();
                if self.sign(end) { break; }
            }
            self.need_sign(end, "after the binding targets")?;
            return Ok(forms);
        }
        let token = self.look().clone();
        self.unsupported_place = false;
        // A bare word bound here is a member where the binding stands
        // in a class body; a place within something binds no word.
        let next = self.glance(1).clone();
        let within = table.spells("op.pipe", &next.lexeme)
            || (next.shape == Shape::Sign && ["syntax.group.open", "syntax.array.open", "syntax.call.open"].iter().any(|label| table.spells(label, &next.lexeme)));
        if token.shape == Shape::Bare && !within { self.claim(&token.lexeme); }
        let target = self.deletion_place()?;
        if self.unsupported_place {
            return Ok(vec![prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.stmt.binding.unrun").unwrap_or_default()))])]);
        }
        let previous = self.waiting.replace(source.to_string());
        let written = self.write_into(target, false, None, token);
        self.waiting = previous;
        Ok(vec![written?])
    }

    /// Whether the brackets opening here hold what a place is reached
    /// through rather than a list of targets: past the bracket that
    /// closes them stands the member mark or an index, as in the
    /// writing `del (a).b` and `del (a)[0]`.
    fn bracketed_holder(&self) -> bool {
        if !self.on_any("syntax.group.open") { return false; }
        let (open, close) = match (self.table.single("syntax.group.open"), self.table.single("syntax.group.close")) {
            (Some(open), Some(close)) => (open.to_string(), close.to_string()),
            _ => return false,
        };
        let mut still = 0usize;
        for step in self.pos..self.tokens.len() {
            let word = &self.tokens[step];
            if word.shape != Shape::Sign { continue; }
            if word.lexeme == open { still += 1; continue; }
            if word.lexeme != close { continue; }
            still -= 1;
            if still > 0 { continue; }
            let past = match self.tokens.get(step + 1) { Some(past) => past, None => return false };
            return past.shape == Shape::Sign
                && (self.table.spells("op.pipe", &past.lexeme) || self.table.spells("op.index.open", &past.lexeme));
        }
        false
    }

    fn deletion_place(&mut self) -> Res<Form> {
        let reached = if self.bracketed_holder() {
            let close = self.table.single("syntax.group.close").unwrap_or_default().to_string();
            self.advance();
            let inner = self.expr(0)?;
            self.need_sign(&close, "after the bracketed target")?;
            inner
        } else {
            let name = self.need_word("as a binding target")?;
            self.read(&name)
        };
        let mut place = self.called_on_value(reached)?;
        loop {
            if self.on_any("op.pipe") {
                self.advance();
                let field = self.need_word("after the member mark")?;
                place = prim_call(Prim::Of, vec![place, constant(Value::text(&field))]);
                place = self.called_on_value(place)?;
            } else if self.on_any("op.index.open") {
                self.advance();
                if self.table.has_any("ext.stmt.class.special") && self.table.has_any("ext.op.index.slice") {
                    let end = self.table.single("op.index.close").unwrap().to_owned();
                    let separator = self.table.single("syntax.call.separator").map(str::to_owned);
                    let mut keys = vec![self.bracket_part(&end, separator.as_deref())?];
                    let mut several = false;
                    while separator.as_ref().map_or(false, |word| self.sign(word)) {
                        several = true;
                        self.advance();
                        if self.sign(&end) { break; }
                        keys.push(self.bracket_part(&end, separator.as_deref())?);
                    }
                    self.need_sign(&end, "after the index")?;
                    // Places written with commas between are one key
                    // holding them all, where the table has slice
                    // values; elsewhere no such place can be written to.
                    let key = if several && self.table.has_any("ext.builtin.slice") {
                        prim_call(Prim::MakeTuple, keys)
                    } else {
                        if several { self.unsupported_place = true; }
                        keys.swap_remove(0)
                    };
                    place = prim_call(Prim::At, vec![place, key]);
                    continue;
                }
                let mut indices = Vec::new();
                let mut special = false;
                while !self.on_any("op.index.close") && !self.exhausted() {
                    if self.on_any("block.intro") || self.on_any("syntax.call.separator") {
                        special = true;
                        self.advance();
                    } else {
                        if self.on_any("op.pipe") && self.glance(1).lexeme == self.look().lexeme && self.glance(2).lexeme == self.look().lexeme {
                            self.advance(); self.advance(); self.advance();
                            indices.push(constant(Value::Nil));
                            special = true;
                        } else { indices.push(self.expr(0)?); }
                        if !self.on_any("block.intro") && !self.on_any("syntax.call.separator") { break; }
                    }
                }
                self.need_sign(self.table.single("op.index.close").unwrap(), "after the index")?;
                let key = if special || indices.len() != 1 {
                    self.unsupported_place = true;
                    constant(Value::Small(0))
                } else { indices.pop().unwrap() };
                place = prim_call(Prim::At, vec![place, key]);
            } else { return Ok(place); }
        }
    }

    fn global_names(&mut self) -> Res<Form> {
        self.advance();
        let said_at = self.pos - 1;
        let sep = self.table.single("syntax.call.separator").map(str::to_string);
        let mut ready = Vec::new();
        loop {
            // A name worked out as the run goes already spells one of
            // the outermost bindings, which is what a global is, so
            // saying so binds nothing further: only the name itself is
            // worked out, and what it spells made ready to be read.
            if self.look().shape != Shape::Bare {
                let seen = self.look().lexeme.clone();
                match self.expr(0)? {
                    Form::Called(spells) => ready.push(Form::ReadyCalled(spells)),
                    _ => return Err(format!("Expected identifier after the global keyword, got '{}'", seen)),
                }
            } else {
                let name = self.need_word("after the global keyword")?;
                // A name bound to a global names it whether or not
                // anything was ever written there, so the global is made
                // to hold nothing where it held nothing at all: reading
                // it is then reading a binding written to.
                let at = self.global_address(&name);
                ready.push(Form::Ready(at));
                // A class body pushes no layer of its own, so the
                // layer around it is not where this belongs: put there,
                // the declaration would outlive the body, reaching the
                // rest of the layer's own code and every routine nested
                // in the body, none of which a class's `global` ever
                // reaches in the reference. It is kept instead against
                // the body itself, at the depth that names it, so only
                // the body's own statements -- standing at that same
                // depth, never a routine entered from within it -- ever
                // find it there.
                if self.in_class_body() {
                    if self.table.has_any("ext.builtin.exceptions.syntax") {
                        let kept = &self.class_globals.last().expect("the class body").1;
                        let met = &self.class_met.last().expect("the class body");
                        let met_kind = if kept.iter().any(|n| *n == name) { None }
                            else { [MET_READ, MET_ANNOTATED, MET_WRITTEN].into_iter().find(|kind| met.iter().any(|(n, k)| n == &name && k == kind)) };
                        if let Some(kind) = met_kind {
                            let complaint = match kind {
                                MET_READ => format!("name '{name}' is used prior to global declaration"),
                                MET_ANNOTATED => format!("annotated name '{name}' can't be global"),
                                _ => format!("name '{name}' is assigned to before global declaration"),
                            };
                            let mut last = &self.tokens[said_at];
                            for t in self.tokens.iter().skip(said_at) {
                                if matches!(t.shape, Shape::LineEnd | Shape::Close | Shape::Finish) || t.lexeme == ";" { break; }
                                last = t;
                            }
                            self.range_end = Some((last.column + last.lexeme.chars().count(), last.row));
                            self.pos = said_at;
                            return Err(format!("SyntaxError: {complaint}"));
                        }
                    }
                    self.class_globals.last_mut().expect("the class body").1.push(name);
                } else {
                    let owner = self.layers.iter_mut().rev().find(|s| s.holds == Holds::Every).expect("the top layer");
                    owner.aliases.push((name.clone(), name));
                }
            }
            match &sep {
                Some(s) if self.sign(s) => self.advance(),
                _ => {
                    ready.push(constant(Value::Nil));
                    return Ok(sequence(ready));
                }
            };
        }
    }

    /// `static x = e;`: x means a hidden global, set where the function
    /// is defined, so it keeps its value between calls. The setting is
    /// read outside the function's layer, where it will run. Outside
    /// every function there is no layer around this one, so the setting
    /// stands where it is written and is guarded: it happens the first
    /// time the statement is reached and no other time.
    ///
    /// Text handed over while the run goes is read afresh each time it
    /// is reached, so a statement at the top of such text has no run of
    /// calls to keep anything across. There the statement is nothing
    /// but a write of the name where the text was read
    /// (ext.stmt.static.read_in), and what it wrote stays as anything
    /// else written to that name stays.
    fn static_names(&mut self) -> Res<Form> {
        self.advance();
        let alone = self.layers.len() < 2;
        let handed_over = self.read_in && self.layers.len() == self.outer_layers && self.table.flag("ext.stmt.static.read_in");
        let sep = self.table.single("syntax.call.separator").map(str::to_string);
        let mut here = Vec::new();
        loop {
            let name = self.need_word("after the static keyword")?;
            if handed_over {
                // A name is bound once here as anywhere: nothing is
                // left behind to show one said twice, so the names
                // spoken for are gathered and counted on their own.
                if self.spoken_for.iter().any(|n| *n == name) {
                    self.stopped_fatally = true;
                    return Err(format!("Duplicate declaration of static variable {}", name));
                }
                self.spoken_for.push(name.clone());
                let value = if self.on_assign() {
                    self.advance();
                    self.expr(0)?
                } else {
                    constant(Value::Nil)
                };
                here.push(Form::Write(self.address_to_write(&name), Box::new(value)));
                match &sep {
                    Some(s) if self.sign(s) => {
                        self.advance();
                        continue;
                    }
                    _ => return Ok(sequence(here)),
                }
            }
            // One name kept between calls is one binding: saying so
            // twice in one program is a thing the language refuses.
            let owner = self.layers.iter().rev().find(|s| s.holds == Holds::Every).expect("the top layer");
            if owner.aliases.iter().any(|(n, _)| *n == name) {
                self.stopped_fatally = true;
                return Err(format!("Duplicate declaration of static variable {}", name));
            }
            self.gensyms += 1;
            let hidden = format!("#static{}", self.gensyms);
            let inner = if alone { None } else { Some(self.layers.pop().expect("the function's layer")) };
            let value = if self.on_assign() {
                self.advance();
                self.expr(0)
            } else {
                Ok(constant(Value::Nil))
            };
            let slot = self.global_address(&hidden);
            if let Some(inner) = inner {
                self.layers.push(inner);
            }
            let written = Form::Write(slot.clone(), Box::new(value?));
            match alone {
                true => {
                    let once = self.choose(Form::Missing(slot), written, constant(Value::Nil));
                    here.push(once);
                }
                false => self.statics.push(written),
            }
            let owner = self.layers.iter_mut().rev().find(|s| s.holds == Holds::Every).expect("the outermost layer");
            owner.aliases.push((name, hidden));
            match &sep {
                Some(s) if self.sign(s) => self.advance(),
                _ => return Ok(sequence(here)),
            };
        }
    }

    /// Statements separated as call arguments are, up to a closing sign.
    fn clauses(&mut self, close: &str) -> Res<Form> {
        let sep = self.table.single("syntax.call.separator").map(str::to_string);
        let mut items = Vec::new();
        while !self.lexeme_of(close) && !self.exhausted() {
            items.push(self.plain_stmt()?);
            match &sep {
                Some(s) if self.sign(s) => self.advance(),
                _ => break,
            };
        }
        Ok(sequence(items))
    }

    /// A watched arm written beside its colon ends at the line end;
    /// one written below it follows the ordinary indentation reading.
    fn watched_body(&mut self) -> Res<Form> {
        if self.table.has_any("ext.builtin.exceptions.syntax") {
            if !self.on_any("block.intro") { return Err(String::from("SyntaxError: expected ':'")); }
        }
        // Standing in a class body, the arm names members and is read
        // the way the body around it is read.
        if self.in_class_body() { return self.class_limb(); }
        if self.table.blocks != Blocks::Indented || !self.on_any("block.intro") {
            return self.body();
        }
        self.advance();
        if self.look().shape == Shape::LineEnd { return self.body(); }
        let mut words = vec![self.stmt()?];
        while self.on_any("stmt.terminator") {
            self.advance();
            if matches!(self.look().shape, Shape::LineEnd | Shape::Finish) { break; }
            words.push(self.stmt()?);
        }
        Ok(sequence(words))
    }

    /// `try { } catch (A | B $e) { } finally { }`: the body is watched,
    /// the first clause whose class it raises takes it, and the last
    /// part runs however the body ended, so a return leaves through it.
    fn guarded_attempt_body(&mut self, bare: bool) -> Res<Form> {
        self.syntax_try_nesting += 1;
        let result = if bare { self.watched_body() } else { self.body() };
        self.syntax_try_nesting -= 1;
        result
    }

    fn attempt_stmt(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
        let bare_clauses = table.single("ext.stmt.catch.as").is_some();
        let body = self.guarded_attempt_body(bare_clauses)?;
        let open = table.single("syntax.group.open").ok_or_else(|| "A catch needs syntax.group".to_string())?.to_string();
        let close = table.single("syntax.group.close").unwrap().to_string();
        let mut clauses: Vec<Clause> = Vec::new();
        let mut all_at = None;
        let mut all_end = None;
        // A clause may stand on a line of its own, after what it follows.
        self.skip_line_ends();
        while self.key("ext.stmt.catch") {
            if table.has_any("ext.builtin.exceptions.syntax") {
                if let Some(begin) = all_at { self.pos = begin; self.range_end = all_end; return Err("SyntaxError: default 'except:' must be last".to_string()); }
            }
            let origin = self.pos;
            self.advance();
            let mut classes = Vec::new();
            let mut choices = None;
            let grouped = bare_clauses && self.on_any("ext.stmt.catch.group");
            if grouped { self.advance(); }
            if table.has_any("ext.builtin.exceptions.syntax") {
                if grouped && self.on_any("block.intro") { return Err("SyntaxError: expected one or more exception types".to_string()); }
                if clauses.first().map_or(false, |c| grouped != c.grouped) {
                    let after = &self.tokens[self.pos - 1];
                    self.range_end = Some((after.column + after.lexeme.chars().count(), after.row));
                    self.pos = origin;
                    return Err(table.single("ext.stmt.catch.amiss").unwrap_or_default().to_owned());
                }
            }
            let held;
            let mut takes_all = false;
            if bare_clauses {
                let mut selectors = Vec::new();
                let bracketed = self.on_any("ext.stmt.catch.tuple.open");
                if bracketed { self.advance(); }
                takes_all = !bracketed && self.on_any("block.intro");
                if !takes_all && !(bracketed && self.on_any("ext.stmt.catch.tuple.close")) {
                    loop {
                        let selector = match self.expr(0)? {
                            Form::Read(place) if !table.has_any("ext.builtin.exceptions.syntax") => Form::Glance(place),
                            other => other,
                        };
                        selectors.push(selector);
                        if !self.on_any("ext.stmt.catch.separator") { break; }
                        self.advance();
                        if bracketed && self.on_any("ext.stmt.catch.tuple.close") { break; }
                    }
                }
                if bracketed {
                    self.need_sign(table.single("ext.stmt.catch.tuple.close").unwrap_or(")"), "after the classes caught")?;
                }
                if table.has_any("ext.builtin.exceptions.syntax") && !bracketed && selectors.len() > 1
                    && self.key("ext.stmt.catch.as") {
                    return Err(String::from("SyntaxError: multiple exception types must be parenthesized when using 'as'"));
                }
                held = if self.key("ext.stmt.catch.as") {
                    self.advance();
                    if grouped && table.has_any("ext.builtin.exceptions.syntax") {
                        let target = if self.on_any("syntax.group.open") { Some("tuple") }
                            else if matches!(self.look().shape, Shape::Numeral | Shape::Quote) { Some("literal") }
                            else { None };
                        if let Some(kind) = target { return Err(format!("SyntaxError: cannot use except* statement with {kind}")); }
                    }
                    let start = self.pos;
                    let binding = self.need_word("after the caught value's binding word")?;
                    if table.has_any("ext.builtin.exceptions.syntax") && binding == "__debug__" {
                        return Err(String::from("SyntaxError: cannot assign to __debug__"));
                    }
                    if table.has_any("ext.builtin.exceptions.syntax") {
                        let kind = match self.look().lexeme.as_str() { "." => Some("attribute"), "[" => Some("subscript"), _ => None };
                        if let Some(kind) = kind {
                            while !self.on_any("block.intro") && !self.on_stmt_end() && !self.exhausted() { self.advance(); }
                            let tail = &self.tokens[self.pos - 1];
                            self.range_end = Some((tail.column + tail.lexeme.chars().count(), tail.row));
                            self.pos = start;
                            let clause = if grouped { "except*" } else { "except" };
                            return Err(format!("SyntaxError: cannot use {clause} statement with {kind}"));
                        }
                    }
                    self.claim(&binding);
                    Some(self.address_to_write(&binding))
                } else { None };
                choices = Some(selectors);
            } else {
                self.need_sign(&open, "after catch")?;
                classes.push(self.need_word("as the class caught")?);
                while table.single("ext.stmt.catch.separator").map_or(false, |s| self.sign(s)) {
                    self.advance();
                    classes.push(self.need_word("as another class caught")?);
                }
                held = if self.look().shape == Shape::Bare {
                    let binding = self.advance().lexeme;
                    Some(self.address_to_write(&binding))
                } else { None };
                self.need_sign(&close, "after the class caught")?;
            }
            let body = self.guarded_attempt_body(bare_clauses)?;
            // A class body lets the caught value's name go when the
            // clause is through with it, as the language this follows
            // does, and so keeps no member under that name.
            let body = match (&held, self.in_class_body()) {
                (Some(place), true) => sequence(vec![body, Form::Forget(place.clone())]),
                _ => body,
            };
            if takes_all {
                all_at = Some(origin);
                for token in self.tokens[origin..self.pos].iter().rev() {
                    if matches!(token.shape, Shape::Open | Shape::Close | Shape::LineEnd) { continue; }
                    all_end = Some((token.column + token.lexeme.chars().count(), token.row)); break;
                }
            }
            clauses.push(Clause { source_line: self.tokens[origin].row.saturating_sub(self.before), classes, choices, grouped, takes_all, held, body });
            self.skip_line_ends();
        }
        // Where the table has words for it, grouped clauses may not
        // stand beside plain ones, and each must name its classes.
        if let Some(amiss) = table.single("ext.stmt.catch.amiss") {
            let with_star = clauses.iter().filter(|c| c.grouped).count();
            if with_star > 0 && (with_star < clauses.len() || clauses.iter().any(|c| c.grouped && c.takes_all)) {
                return Err(amiss.to_string());
            }
        }
        let otherwise = if table.flag("ext.stmt.try.else") && self.key("stmt.else") {
            self.advance();
            let limb = self.guarded_attempt_body(true)?;
            self.skip_line_ends();
            Some(Box::new(limb))
        } else { None };
        let last = match self.key("ext.stmt.finally") {
            true => {
                self.advance();
                self.finally_nesting += 1;
                let body = self.guarded_attempt_body(bare_clauses)?;
                self.finally_nesting -= 1;
                Some(Box::new(body))
            }
            false => None,
        };
        if clauses.is_empty() && (last.is_none() || otherwise.is_some()) {
            let said = if table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: expected 'except' or 'finally' block" }
                else { "A try needs a catch or a last part" };
            return Err(said.to_string());
        }
        Ok(Form::Attempt { context: None, async_context: false, body: Box::new(body), clauses, last, otherwise })
    }

    /// A class and what it holds: properties, constants, values kept by
    /// the class, and methods. The class becomes a value under its own
    /// name, so `new C` and `C::X` are ordinary reads.
    /// A bag of members a class may take in as its own. Nothing of it
    /// runs and nothing is bound to its name: where its body begins is
    /// written down, and every class taking it in reads that body again
    /// as its own, so the members stand in the class and not in the bag.
    fn bag_decl(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
        let name = self.need_word("as the name of the members to take in")?;
        self.skip_lead_word();
        self.skip_line_ends();
        let opens = table.strings("block.open");
        let k = opens
            .iter()
            .position(|o| self.lexeme_of(o))
            .ok_or_else(|| format!("Expected '{}' to open the members, got '{}'", opens[0], self.look().lexeme))?;
        self.advance();
        self.bags.insert(name, self.pos);
        // The body is stepped over, mark for mark, since it is read
        // where it is taken in and nowhere else.
        let (open, close) = (opens[k].clone(), table.strings("block.close")[k].clone());
        let mut deep = 1usize;
        while deep > 0 {
            if self.exhausted() {
                return Err(format!("Expected '{}' to close the members", close));
            }
            if self.lexeme_of(&open) {
                deep += 1;
            } else if self.lexeme_of(&close) {
                deep -= 1;
            }
            self.advance();
        }
        Ok(constant(Value::Nil))
    }

    /// `use T, U { T::f as g; }`: the members of each bag named are read
    /// again here, standing in the class that takes them in, and any of
    /// them may be given another name in it as well.
    fn take_in_members(&mut self, held: &mut Members) -> Res<()> {
        let table = self.table;
        self.advance();
        let apart = table.single("syntax.call.separator").map(str::to_string);
        let mut taken = Vec::new();
        loop {
            taken.push(self.need_word("as the members to take in")?);
            match &apart {
                Some(sep) if self.sign(sep) => self.advance(),
                _ => break,
            };
        }
        for named in &taken {
            let Some(from) = self.bags.get(named).copied() else {
                return Err(format!("No members named {} to take in", named));
            };
            let close = table.strings("block.close")[0].clone();
            let stood = self.pos;
            self.pos = from;
            let read = self.class_members(&close, held);
            self.pos = stood;
            read?;
        }
        // What follows may name members of the bags and give each
        // another name in this class.
        let opens = table.strings("block.open");
        let Some(k) = opens.iter().position(|o| self.lexeme_of(o)) else { return Ok(()) };
        self.advance();
        let close = table.strings("block.close")[k].clone();
        self.skip_line_ends();
        while !self.lexeme_of(&close) && !self.exhausted() {
            let first = self.need_word("as the member to name again")?;
            let member = match table.single("ext.op.scope").map_or(false, |m| self.sign(m)) {
                true => {
                    self.advance();
                    self.need_word("as the member to name again")?
                }
                false => first,
            };
            if !self.key("ext.stmt.class.uses.alias") {
                return Err(format!("Expected '{}' after the member to name again", table.single("ext.stmt.class.uses.alias").unwrap_or("as")));
            }
            self.advance();
            let called = self.need_word("as the other name")?;
            let found = held.methods.iter().find(|(n, _)| *n == member).map(|(_, p)| p.clone());
            let Some(program) = found else {
                return Err(format!("No member named {} among the ones taken in", member));
            };
            held.methods.push((called, program));
            self.skip_line_ends();
        }
        self.need_lexeme(&close)?;
        Ok(())
    }

    /// The words a target hands a value to, in the order it hands them.
    /// Where the target carries anything besides words, the marks that
    /// join them, brackets and the rest mark, it hands nothing to a
    /// member and none of it is gathered.
    fn target_words(&self, span: std::ops::Range<usize>, gathered: &mut Vec<String>) -> bool {
        if span.start >= span.end { return false; }
        for at in span {
            let token = &self.tokens[at];
            let punctuation = ["ext.op.tuple", "ext.stmt.unpack.rest", "syntax.group.open", "syntax.group.close",
                "syntax.array.open", "syntax.array.close"];
            match token.shape {
                Shape::Bare if !self.table.keywords.contains(&token.lexeme) => gathered.push(token.lexeme.clone()),
                Shape::Sign if punctuation.iter().any(|label| self.table.spells(label, &token.lexeme)) => {}
                _ => return false,
            }
        }
        true
    }

    /// Members a body binds by taking a value apart (`a, b = 1, 2`) or
    /// through a chain of signs (`x = y = 3`). Each of these names more
    /// members than the one member the walk over a body knows how to
    /// keep, and each works its value out once before handing it to
    /// every target in turn. Nothing comes back where the statement is
    /// neither form, and nothing has been read, so the walk goes on to
    /// the readers that come after this one.
    fn taken_apart_members(&mut self, setup: &mut Vec<Form>) -> Res<Option<Vec<(String, Address)>>> {
        let mut begins = self.pos;
        let signs = self.divided_at(begins, self.tokens.len(), "stmt.assign");
        if signs.is_empty() || (signs.len() > 1 && !self.table.flag("ext.stmt.assign.chain")) { return Ok(None); }
        // A single word before a single sign is the member the walk
        // already keeps, so it is left to the reader that keeps it.
        if signs.len() == 1 && signs[0] == begins + 1 { return Ok(None); }
        let mut targets = Vec::new();
        for sign in &signs {
            targets.push(begins..*sign);
            begins = sign + 1;
        }
        let mut gathered = Vec::new();
        if targets.iter().any(|span| !self.target_words(span.clone(), &mut gathered)) { return Ok(None); }
        if gathered.is_empty() { return Ok(None); }
        // The value is worked out while the words still stand for what
        // the body bound earlier, so that `a, b = a + 1, 2` reads the
        // member `a` held rather than the place about to replace it.
        self.pos = signs[signs.len() - 1] + 1;
        let answer = match self.table.has_any("ext.op.tuple") {
            true => self.comma_value()?,
            false => self.expr(0)?,
        };
        let resume = self.pos;
        let source = self.gensym("given").ident.to_string();
        setup.push(self.write(&source, answer));
        // Now each word is given a place belonging to the class. The
        // handing out below is the ordinary one, and it looks the words
        // up as any reading in a body does, so what a target receives
        // lands in these places and not in the scope around the class.
        let mut kept = Vec::new();
        for word in gathered {
            let place = self.member_address(&word, "attribute");
            self.class_bindings.last_mut().expect("the class namespace").1.insert(word.clone(), place.clone());
            kept.push((word, place));
        }
        for span in targets {
            let handed = self.distribute(span, &source)?;
            setup.push(handed);
        }
        self.pos = resume;
        Ok(Some(kept))
    }

    /// The place a member's value is kept in. A name the body has
    /// already used keeps the place it was given, so an arm of a
    /// conditional writes where the rest of the body reads; a name new
    /// to the body is given a place of its own.
    fn member_address(&mut self, word: &str, purpose: &str) -> Address {
        if !self.importing { self.mark_met(word, MET_WRITTEN); }
        if !self.parts().lexical_members.iter().any(|entry| entry == word) {
            self.parts().lexical_members.push(word.to_owned());
        }
        match self.class_bindings.last().and_then(|(_, names)| names.get(word)).cloned() {
            Some(place) => place,
            None => self.gensym(purpose),
        }
    }

    /// A name taking its stand in the namespace of the class. A body
    /// that binds one name twice leaves it where it first stood, as a
    /// second write to a map leaves a key where it was; a body that
    /// takes a name away loses that stand, and binding the name once
    /// more puts it behind all the others.
    fn member_ranked(&mut self, word: &str) {
        let parts = self.parts();
        if parts.ranking.iter().all(|old| old != word) { parts.ranking.push(word.to_string()); }
    }

    /// A member entered under its name, in the place given, and made
    /// known by that name to the rest of the body. One entered from
    /// within an arm is noted as a member whose place may stay unwritten,
    /// and it displaces any method of that name, since the arm decides
    /// which of the two the class ends up with.
    fn member_noted(&mut self, word: &str, place: Address) {
        self.member_ranked(word);
        let parts = self.parts();
        if let Some(at) = parts.attributes.iter().position(|old| old == word) {
            parts.attributes.remove(at);
            parts.held.remove(at);
        }
        if parts.arms > 0 {
            parts.methods.retain(|(old, _)| old != word);
            if !parts.uncertain.iter().any(|n| n == word) { parts.uncertain.push(word.to_string()); }
        }
        parts.attributes.push(word.to_string());
        parts.held.push(Form::Read(place.clone()));
        self.class_bindings.last_mut().expect("the class namespace").1.insert(word.to_string(), place);
    }

    /// One arm of a conditional in a class body, read the way the body
    /// around it is read. An arm may stand on the line of its own head,
    /// as any block may.
    fn class_limb(&mut self) -> Res<Form> {
        let head_mark = self.on_any("block.intro");
        self.skip_lead_word();
        let mut items = Vec::new();
        if head_mark && self.table.flag("ext.block.lone_statement") && !self.on_stmt_end()
            && !matches!(self.look().shape, Shape::Open | Shape::Close | Shape::Finish) {
            loop {
                let gone_past = self.class_item(&mut items)?;
                if gone_past { break; }
                if !(self.look().shape == Shape::Sign && self.table.separates(&self.look().lexeme)) { break; }
                self.advance();
                if matches!(self.look().shape, Shape::Finish | Shape::Close | Shape::LineEnd) { break; }
            }
            return Ok(sequence(items));
        }
        self.skip_line_ends();
        if self.look().shape != Shape::Open { return Err("Expected an indented class body".to_string()); }
        self.advance();
        self.skip_line_ends();
        while self.look().shape != Shape::Close && !self.exhausted() {
            self.class_item(&mut items)?;
            self.skip_line_ends();
        }
        if self.look().shape != Shape::Close { return Err("Expected the end of a class body".to_string()); }
        self.advance();
        Ok(sequence(items))
    }

    /// A conditional in a class body. Whichever arm runs names members
    /// of the class, so the names of every arm are gathered together and
    /// each is given one place that every arm naming it writes into. A
    /// place no arm reached holds nothing and the class is given no
    /// member for it, since a name a conditional never bound is no
    /// member of the class in the language this follows.
    fn class_choice(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
        let test = self.expr(0)?;
        self.parts().arms += 1;
        let taken = self.class_limb();
        self.parts().arms -= 1;
        let taken = taken?;
        // A further arm may follow the ends of lines.
        let mut ahead = 0;
        while self.glance(ahead).shape == Shape::LineEnd
            || (self.glance(ahead).shape == Shape::Sign && table.separates(&self.glance(ahead).lexeme)) {
            ahead += 1;
        }
        let next = self.glance(ahead);
        let further = next.shape == Shape::Bare && table.spells("stmt.elif", &next.lexeme);
        let remaining = next.shape == Shape::Bare && table.spells("stmt.else", &next.lexeme);
        if !further && !remaining {
            return Ok(self.choose(test, taken, constant(Value::Nil)));
        }
        self.pos += ahead;
        if remaining { self.advance(); }
        self.parts().arms += 1;
        let untaken = match further || self.key("stmt.if") {
            true => self.class_choice(),
            false => self.class_limb(),
        };
        self.parts().arms -= 1;
        Ok(self.choose(test, taken, untaken?))
    }

    /// One member of a class body. True where the member has already
    /// gone past whatever follows it, so the walk leaves that alone.
    fn class_item(&mut self, setup: &mut Vec<Form>) -> Res<bool> {
        let table = self.table;
        if !table.has_any("ext.stmt.class.detail.root") && self.on_any("ext.stmt.decorator") {
            let (word, address) = self.member_adornments(setup)?;
            self.parts().cannot |= table.single("ext.stmt.class.constructor") == Some(word.as_str());
            self.parts().methods.retain(|(n, _)| n != &word);
            self.member_noted(&word, address);
            self.skip_line_ends();
            return Ok(true);
        }
        let mut wrappers=Vec::new();
        while table.has_any("ext.stmt.class.detail.root") && self.on_any("ext.stmt.decorator") {
            self.advance();
            if !self.adornment_head_ok() { return Err(table.single("ext.stmt.decorator.amiss").unwrap_or_default().to_string()); }
            let expression=self.expr_at(0,false)?;let address=self.gensym("member_wrapper");
            setup.push(Form::Write(address.clone(),Box::new(expression)));wrappers.push(address);
            self.skip_line_ends();
        }
        if self.key("ext.stmt.async") && table.spells("stmt.function", &self.glance(1).lexeme) { self.advance(); self.coroutine_next = true; }
        if !wrappers.is_empty() && !self.key("stmt.function") && !self.key("ext.stmt.class") {self.parts().cannot=true;}
        if self.key("stmt.function") {
            self.advance();
            let method_name = self.need_word("as the method name")?;
            let title = self.original_words[self.pos - 1].lexeme.clone();
            let body = self.method(&title)?;
            self.parts().methods.retain(|(old, _)| old != &method_name);
            if let Some(at) = self.parts().attributes.iter().position(|old| old == &method_name) {
                self.parts().attributes.remove(at);
                self.parts().held.remove(at);
            }
            let slot = self.member_address(&method_name, "method_body");
            let decorated=!wrappers.is_empty();
            let mut expression=constant(Value::Routine(body.clone()));
            while let Some(address)=wrappers.pop(){expression=Form::Apply(Callee::Code(Box::new(Form::Read(address))),vec![expression]);}
            setup.push(Form::Write(slot.clone(),Box::new(expression)));
            // A method of a class standing in a function climbs through
            // that function's frame to reach every name outside its own,
            // the module's included. The write of its address binds it to
            // that frame as the statement runs; the plan's copy is bound
            // to nothing, so the method is handed over from its address.
            // Text handed over while the run goes stands in a function
            // this way too, even though the layer for the frame around
            // it is marked `Fresh` rather than counted here as one of
            // the routines: a class read there is exactly a class
            // standing in a function, and its methods need the same
            // binding to climb out through that frame.
            let stands_in_routine = self.layers.iter().filter(|s| s.holds == Holds::Every).count() > 1
                || (self.read_in && self.outer_layers > 1);
            // A method an arm of a conditional writes belongs among the
            // members the class is handed, not among the methods it
            // always has: the arm holding it may never run.
            match decorated || self.parts().arms > 0 || stands_in_routine {
                true => self.member_noted(&method_name, slot.clone()),
                false => {
                    self.class_bindings.last_mut().expect("the class namespace").1.insert(method_name.clone(), slot.clone());
                    self.member_ranked(&method_name);
                    self.parts().methods.push((method_name.clone(), body));
                }
            }
            if let Some(mirror) = self.mirror_member(&method_name, &slot) { setup.push(mirror); }
        } else if self.key("stmt.pass") || self.look().shape == Shape::Quote {
            self.advance();
        } else if table.blocks == Blocks::Indented && self.key("stmt.if") {
            let chosen = self.class_choice()?;
            setup.push(chosen);
        } else if table.blocks == Blocks::Indented && (self.key("stmt.for") || self.key("stmt.while")
            || self.key("ext.stmt.try") || self.key("ext.stmt.with")) {
            let block = self.class_block()?;
            setup.push(block);
        } else if table.blocks == Blocks::Indented && (self.key("ext.stmt.import") || self.key("ext.stmt.import.from")) {
            let brought = self.class_import()?;
            setup.push(brought);
        } else if table.blocks == Blocks::Indented && self.key("ext.stmt.del") {
            let removal = self.class_removal()?;
            setup.push(removal);
        } else if let Some(kept) = self.taken_apart_members(setup)? {
            for (word, place) in kept {
                self.member_noted(&word, place.clone());
                if let Some(mirror) = self.mirror_member(&word, &place) { setup.push(mirror); }
            }
        } else {
            let member = self.look().lexeme.clone();
            let value = if self.key("ext.stmt.class") {
                let member = self.glance(1).lexeme.clone();
                setup.push(self.class_with_receiver()?.0);
                let mut nested = self.read(&member);
                while let Some(address) = wrappers.pop() { nested = Form::Apply(Callee::Code(Box::new(Form::Read(address))), vec![nested]); }
                Some((member, nested))
            } else if self.look().shape == Shape::Bare && table.spells("stmt.assign", &self.glance(1).lexeme) {
                self.pos += 2;
                // Commas after the value gather a tuple for the member,
                // where the language has them.
                let gathered = table.has_any("ext.op.tuple");
                let value = if gathered { self.comma_value()? } else { self.expr(0)? };
                Some((member, value))
            } else if self.look().shape == Shape::Bare && !table.keywords.contains(&self.look().lexeme)
                && table.spells("ext.stmt.annotation", &self.glance(1).lexeme) {
                // A keyword ahead of the mark, as `try:`, begins a
                // statement of the body, not an annotated member.
                self.pos += 2;
                self.parts().lexical_members.push(member.clone());
                // The annotation is kept as a routine answering its value,
                // worked out only once the class's annotations are asked
                // for, when a name the body had not yet bound -- the class
                // itself among them -- is bound.
                let deferred = self.annotation_routine()?;
                let key = self.private_key(&member);
                self.parts().annotated_names.push((constant(Value::text(&key)), deferred));
                if self.on_assign() {
                    self.advance();
                    let value = self.comma_value()?;
                    Some((member, value))
                } else { None }
            } else if self.look().shape == Shape::Bare && !table.keywords.contains(&self.look().lexeme)
                && self.glance(1).shape == Shape::Sign && table.compound.contains_key(&self.glance(1).lexeme) {
                // `x += 1`: x is read as the body reads it, out of the
                // class's place where the body has bound it and out of
                // the scope around the class where it has not, and what
                // the working makes is a member either way, since the
                // language this follows looks x up just so and keeps
                // the answer in the class.
                let word = self.advance().lexeme;
                let sign = self.advance().lexeme;
                let op = self.compound_sign(&sign).expect("a compound sign");
                let current = self.read(&word);
                let added = if table.has_any("ext.op.tuple") { self.comma_value()? } else { self.expr(0)? };
                Some((word, self.kept_after(prim_call(op, vec![current, added]))))
            } else {
                // Whatever else stands here. A statement binding no
                // name -- a call made for its effect, a write into a
                // place within a member, a raise, an assertion, a break
                // or a continue in a loop of the body -- runs as it
                // would anywhere and reads the members as
                // the body does; one whose bindings this walk cannot
                // follow yet is read the same way and the class refused.
                let word = self.look().clone();
                let harmless = match word.shape == Shape::Bare && table.keywords.contains(&word.lexeme) {
                    // `global` and `nonlocal` bind no member: the
                    // name they carry goes on meaning what it always
                    // did, a place beyond the class, so a class body
                    // forms around one exactly as it does around a
                    // call made for its effect.
                    true => ["ext.stmt.throw", "ext.stmt.assert", "stmt.break", "stmt.continue", "ext.stmt.global", "ext.stmt.nonlocal"].iter().any(|label| table.spells(label, &word.lexeme)),
                    false => !self.writes_a_name(self.pos),
                };
                setup.push(self.stmt()?);
                if !harmless { self.parts().cannot = true; }
                None
            };
            if let Some((word, value)) = value {
                // A name the body has declared `global` or `nonlocal`
                // means the place beyond it, as it does anywhere else,
                // and binds no member: the class's own namespace never
                // sees it.
                if self.declared_outside_class(&word) {
                    setup.push(self.write(&word, value));
                } else {
                    let place = self.member_address(&word, "attribute");
                    setup.push(Form::Write(place.clone(), Box::new(value)));
                    self.member_noted(&word, place.clone());
                    if let Some(mirror) = self.mirror_member(&word, &place) { setup.push(mirror); }
                }
            }
        }
        Ok(false)
    }

    /// A loop, a watched statement or a context in a class body. What
    /// their blocks bind are members, as are the loop's own variable,
    /// which the language this follows leaves in the class when the
    /// loop is done, and the context's name. The blocks are read the
    /// way the body around them is read, and each name written in them
    /// gets the body's place for it as it is written, so the write
    /// lands in the class and the member holds what the last pass
    /// wrote. A loop may run no times and a watched block may stop
    /// part way, so everything they name is a member the class may
    /// find unwritten; what a block bound before stopping stays bound,
    /// as it does in the language this follows. A loop target that is
    /// not made of words -- a place within something -- binds no
    /// member, and is refused.
    fn class_block(&mut self) -> Res<Form> {
        if self.key("stmt.for") {
            let cuts = self.divided_at(self.pos + 1, self.tokens.len(), "stmt.for.in");
            let mut words = Vec::new();
            let of_words = cuts.first().map_or(false, |&at| self.target_words(self.pos + 1..at, &mut words));
            if !of_words {
                let read = self.stmt()?;
                self.parts().cannot = true;
                return Ok(read);
            }
        }
        self.parts().arms += 1;
        let looped = self.stmt();
        self.parts().arms -= 1;
        looped
    }

    /// An import in a class body. Each name it binds is a member, the
    /// module or what was taken out of it standing under the name it
    /// was bound to, and nothing of it reaches the scope around the
    /// class. What a star brings out of a module is unknown until the
    /// module is read, so that form stays refused.
    fn class_import(&mut self) -> Res<Form> {
        let table = self.table;
        let starred = (self.pos..self.tokens.len())
            .map(|at| &self.tokens[at])
            .take_while(|word| !matches!(word.shape, Shape::LineEnd | Shape::Finish | Shape::Close))
            .any(|word| word.shape == Shape::Sign && table.spells("op.mul", &word.lexeme));
        let before = self.class_bindings.last().expect("class namespace").1.clone();
        let brought = self.stmt()?;
        let bound = self.class_bindings.last().expect("class namespace").1.clone();
        let mut steps = vec![brought];
        for (word, place) in bound {
            if table.has_any("ext.stmt.class.builder") && !before.contains_key(&word) {
                if let Some(mirror) = self.mirror_member(&word, &place) { steps.push(mirror); }
            }
        }
        let brought = sequence(steps);
        if starred { self.parts().cannot = true; }
        Ok(brought)
    }

    /// `del x` in a class body takes the member out again. Its place is
    /// emptied and the word noted as one the class may find unwritten,
    /// so the class built keeps no member under it; bound afterwards,
    /// it is a member once more. A word the body never bound is no
    /// member to take out: the language this follows stops there
    /// instead of reaching past the class, and the form is refused
    /// here instead of reaching past it. A place within a member --
    /// `del x[0]`, `del x.y` -- is worked as it is anywhere.
    fn class_removal(&mut self) -> Res<Form> {
        let table = self.table;
        let cuts = self.divided_at(self.pos + 1, self.tokens.len(), "syntax.call.separator");
        let mut depth: Vec<String> = Vec::new();
        let mut end = self.tokens.len();
        for at in self.pos + 1..self.tokens.len() {
            let word = &self.tokens[at];
            if depth.is_empty() && (matches!(word.shape, Shape::Finish | Shape::Close | Shape::LineEnd)
                || (word.shape == Shape::Sign && table.separates(&word.lexeme))) { end = at; break; }
            if word.shape != Shape::Sign { continue; }
            if depth.last().map(String::as_str) == Some(word.lexeme.as_str()) { depth.pop(); continue; }
            for family in ["syntax.group", "syntax.array", "syntax.map"] {
                if table.single(&format!("{}.open", family)) == Some(word.lexeme.as_str()) {
                    if let Some(close) = table.single(&format!("{}.close", family)) { depth.push(close.to_string()); }
                    break;
                }
            }
        }
        let mut spans = Vec::new();
        let mut left = self.pos + 1;
        for right in cuts.into_iter().chain(std::iter::once(end)) {
            if left < right { spans.push(left..right); }
            left = right + 1;
        }
        let of_a_word = |span: &std::ops::Range<usize>| span.len() == 1
            && self.tokens[span.start].shape == Shape::Bare && !table.keywords.contains(&self.tokens[span.start].lexeme);
        let words = spans.iter().filter(|span| of_a_word(span)).count();
        if words == 0 { return self.stmt(); }
        if words < spans.len() {
            let read = self.stmt()?;
            self.parts().cannot = true;
            return Ok(read);
        }
        self.advance();
        let mut steps = Vec::new();
        for span in spans {
            let word = self.tokens[span.start].lexeme.clone();
            let known = self.class_bindings.last().and_then(|(_, names)| names.get(&word)).cloned();
            let Some(place) = known else {
                // A name the body never bound at compile time may
                // still be one `locals()[k] = v` bound there while
                // the body ran: `del` of it is a `del` through the
                // body's own namespace all the same, once that
                // namespace exists. With none at all the name is no
                // member to take out, and the language this follows
                // raises NameError rather than reaching past the
                // class into the scope around it.
                let Some(book) = self.parts().book.clone() else {
                    let blank = self.gensym("delete");
                    steps.push(Form::Read(Address { ident: Rc::from(word.as_str()), up: blank.up, at: blank.at, fallback: None }));
                    continue;
                };
                let target = self.read_to_write(&book.ident.to_string());
                let key = constant(Value::text(&word));
                let erased = prim_call(Prim::Erase, vec![target, key]);
                steps.push(self.write(&book.ident.to_string(), erased));
                continue;
            };
            steps.push(Form::Forget(place.clone()));
            let parts = self.parts();
            parts.methods.retain(|(old, _)| old != &word);
            if let Some(at) = parts.attributes.iter().position(|old| old == &word) {
                parts.attributes.remove(at);
                parts.held.remove(at);
            }
            parts.ranking.retain(|old| old != &word);
            parts.attributes.push(word.clone());
            parts.held.push(Form::Read(place));
            if !parts.uncertain.iter().any(|n| n == &word) { parts.uncertain.push(word.clone()); }
            if let Some(mirror) = self.mirror_forget(&word) { steps.push(mirror); }
        }
        self.pos = end;
        Ok(sequence(steps))
    }

    /// The working a compound sign asks for, as the write of a name
    /// does it: on sets where the language has them, on bytes where
    /// bytes are what is written, and asked of the place itself first
    /// where the special list reaches the in-place methods.
    fn compound_sign(&self, sign: &str) -> Option<Prim> {
        let table = self.table;
        let op = *table.compound.get(sign)?;
        let op = match table.flag("ext.syntax.set") {
            true => match op { Prim::BitsBoth => Prim::SetAssign(1), Prim::Minus => Prim::SetAssign(2), Prim::BitsEither => Prim::SetAssign(0), Prim::BitsOne => Prim::SetAssign(3), p => p },
            false => op,
        };
        let op = match table.has_any("ext.builtin.bytes") && matches!(op, Prim::Plus | Prim::Times) {
            true => Prim::OctetAssign(op == Prim::Times),
            false => op,
        };
        Some(table.landing_place(op).map_or(op, Prim::Landing))
    }

    /// Whether the statement starting here binds a name of the scope it
    /// stands in: by a sign binding within an expression, or by a bare
    /// name ahead of a writing sign. A name with a member mark, a
    /// bracket or a call after it is a place within something, as is
    /// the name after a member mark, and a write there binds nothing.
    fn writes_a_name(&self, start: usize) -> bool {
        let table = self.table;
        let mut depth: Vec<String> = Vec::new();
        let mut sign_at = None;
        let mut end = self.tokens.len();
        for at in start..self.tokens.len() {
            let word = &self.tokens[at];
            if depth.is_empty() && (matches!(word.shape, Shape::Finish | Shape::Close | Shape::LineEnd)
                || (word.shape == Shape::Sign && table.separates(&word.lexeme))) { end = at; break; }
            if word.shape == Shape::Sign && table.spells("ext.op.assign.expression", &word.lexeme) { return true; }
            if depth.is_empty() && sign_at.is_none() && word.shape == Shape::Sign
                && (table.spells("stmt.assign", &word.lexeme) || table.compound.contains_key(&word.lexeme)) { sign_at = Some(at); }
            if word.shape != Shape::Sign { continue; }
            if depth.last().map(String::as_str) == Some(word.lexeme.as_str()) { depth.pop(); continue; }
            for family in ["syntax.group", "syntax.array", "syntax.map"] {
                if table.single(&format!("{}.open", family)) == Some(word.lexeme.as_str()) {
                    if let Some(close) = table.single(&format!("{}.close", family)) { depth.push(close.to_string()); }
                    break;
                }
            }
        }
        let Some(sign_at) = sign_at else { return false; };
        // Bare names at the outer level of the target, before the sign.
        // What stands between an annotation's mark and the sign is the
        // annotation itself: its names bind nothing, as the reference
        // reads them.
        let mut annotated = false;
        depth.clear();
        for at in start..sign_at.min(end) {
            let word = &self.tokens[at];
            if depth.is_empty() && word.shape == Shape::Sign && table.spells("ext.stmt.annotation", &word.lexeme) { annotated = true; }
            let after_mark = at > start && table.spells("op.pipe", &self.tokens[at - 1].lexeme);
            if depth.is_empty() && word.shape == Shape::Bare && !annotated && !table.keywords.contains(&word.lexeme) && !after_mark {
                let next = &self.tokens[at + 1];
                let within = table.spells("op.pipe", &next.lexeme)
                    || (next.shape == Shape::Sign && ["syntax.group.open", "syntax.array.open", "syntax.call.open"].iter().any(|label| table.spells(label, &next.lexeme)));
                if !within { return true; }
            }
            if word.shape != Shape::Sign { continue; }
            if depth.last().map(String::as_str) == Some(word.lexeme.as_str()) { depth.pop(); continue; }
            for family in ["syntax.group", "syntax.array", "syntax.map"] {
                if table.single(&format!("{}.open", family)) == Some(word.lexeme.as_str()) {
                    if let Some(close) = table.single(&format!("{}.close", family)) { depth.push(close.to_string()); }
                    break;
                }
            }
        }
        false
    }

    fn class_not_ready(&self) -> Form {
        let said = self.table.single("ext.stmt.class.unready").unwrap_or("This class form cannot run yet");
        prim_call(Prim::Raise, vec![constant(Value::text(said))])
    }

    /// The class header encloses expressions for bases. Only the first
    /// is taken as a parent; the others are read without being run.
    fn member_adornments(&mut self, setup: &mut Vec<Form>) -> Res<(String, Address)> {
        let mut waiting = Vec::new();
        while self.on_any("ext.stmt.decorator") {
            self.advance();
            let mut manner = 'd';
            // With the descriptor protocol in the table, every decorator
            // is an expression applied to the member, the three wrapping
            // builtins with the rest; without it they are told apart.
            let told_apart = self.table.single("ext.stmt.class.detail.descriptor.get").is_none();
            if told_apart && self.glance(1).shape == Shape::LineEnd {
                for (label, mark) in [("ext.stmt.class.static", 's'), ("ext.stmt.class.classmethod", 'c'), ("ext.stmt.class.property", 'p')] {
                    if self.table.spells(label, &self.look().lexeme) { manner = mark; }
                }
            }
            let setter = told_apart && self.table.spells("ext.op.member", &self.glance(1).lexeme)
                && self.table.spells("ext.stmt.class.property.setter", &self.glance(2).lexeme)
                && self.glance(3).shape == Shape::LineEnd;
            let kept = match (manner, setter) {
                (_, true) => {
                    manner = 'w';
                    let word = self.advance().lexeme;
                    self.pos += 2;
                    Some(self.read(&word))
                }
                ('d', false) => Some(self.expr(0)?),
                _ => { self.advance(); None }
            };
            let slot = self.gensym("decoration");
            if let Some(value) = kept { setup.push(Form::Write(slot.clone(), Box::new(value))); }
            waiting.push((manner, slot));
            if self.look().shape != Shape::LineEnd {
                return Err(self.table.single("ext.stmt.decorator.amiss").unwrap_or_default().to_string());
            }
            self.skip_line_ends();
        }
        let word;
        let mut decorated;
        if self.key("ext.stmt.class") {
            word = self.glance(1).lexeme.clone();
            setup.push(self.class_with_receiver()?.0);
            decorated = self.read(&word);
        } else {
            if self.key("ext.stmt.async") { self.advance(); self.coroutine_next = true; }
            if !self.key("stmt.function") {
                return Err(self.table.single("ext.stmt.decorator.amiss").unwrap_or_default().to_string());
            }
            self.advance();
            word = self.need_word("as the method name")?;
            decorated = constant(Value::Routine(self.method(&word)?));
        }
        while let Some((manner, slot)) = waiting.pop() {
            decorated = match manner {
                'd' => invoke(Form::Read(slot), vec![decorated]),
                'w' => prim_call(Prim::Adorn('w'), vec![Form::Read(slot), decorated]),
                other => prim_call(Prim::Adorn(other), vec![decorated]),
            };
        }
        let address = self.gensym("adorned_method");
        setup.push(Form::Write(address.clone(), Box::new(decorated)));
        self.class_bindings.last_mut().expect("the class namespace").1.insert(word.clone(), address.clone());
        Ok((word, address))
    }

    fn python_class(&mut self) -> Res<(Form, bool)> {
        let builder = self.gensym("class_builder");
        let mut setup = vec![Form::Write(builder.clone(), Box::new(prim_call(Prim::ClassWork(14), Vec::new())))];
        self.advance();
        let title = self.original_words[self.pos].lexeme.clone();
        let named = self.need_word("as the class name")?;
        if named == "__debug__" { return Err("SyntaxError: cannot assign to __debug__".into()); }
        if self.on_any("ext.stmt.type_params.open") { self.class_type_parameters()?; }
        let args = if self.table.single("ext.stmt.class.bases.open").map_or(false, |o| self.sign(o)) {
            self.advance(); self.args("syntax.call.close", "syntax.call.separator")?
        } else { Vec::new() };
        let header = self.gensym("class_header");
        setup.push(Form::Write(header.clone(), Box::new(prim_call(Prim::MakeTuple, args))));
        let full_name = self.full_name_of(&title);
        let mut namespace = None;
        let mut cannot = false;
        let body = self.routine(&title, Holds::Every, Traps::Yields, Vec::new(), 0, |b| {
            let (body, book, cell, declined) = b.python_class_body(title.clone(), full_name.clone())?;
            namespace = Some((book, cell)); cannot = declined; Ok(body)
        })?;
        let Form::Const(Value::Routine(code)) = body else { unreachable!() };
        let mut code = (*code).clone(); code.class_namespace = namespace; code.flags &= !3;
        let made = prim_call(Prim::ClassWork(16), vec![Form::Read(builder), constant(Value::Routine(Rc::new(code))), constant(Value::text(&title)), Form::Read(header)]);
        if self.in_class_body() {
            let slot = self.gensym("inner_class");
            self.class_bindings.last_mut().expect("outer class").1.insert(named.clone(), slot.clone());
            setup.push(Form::Write(slot.clone(), Box::new(made)));
            if let Some(mirror) = self.mirror_member(&named, &slot) { setup.push(mirror); }
        } else { setup.push(self.write(&named, made)); }
        Ok((sequence(setup), cannot))
    }

    fn python_class_body(&mut self, class_title: String, full_name: String) -> Res<(Form, String, Option<String>, bool)> {
        let table = self.table;
        let parent: Option<Address> = None;
        let cannot = false;
        let mut setup = Vec::new();
        let previous = self.within.replace((class_title.clone(), parent.as_ref().map(|s| s.ident.to_string())));
        let named_outside = std::mem::replace(&mut self.named_before, self.naming.len());
        if table.has_any("ext.stmt.class.detail.root"){self.within=Some((full_name.clone(),parent.as_ref().map(|a|a.ident.to_string())));}
        self.need_intro()?;
        let on_one_line = !self.on_stmt_end() && self.look().shape != Shape::Open;
        if !on_one_line {
            self.skip_line_ends();
            if self.look().shape != Shape::Open { return Err("Expected an indented class body".to_string()); }
            self.advance();
            self.skip_line_ends();
        }
        let body_source = self.pos;
        let lexical_members = self.surveyed.get(&body_source).map(|scope| scope.bound.clone()).unwrap_or_default();
        self.class_bindings.push((self.layers.len(), HashMap::new()));
        self.class_globals.push((self.layers.len(), Vec::new()));
        self.class_met.push(Vec::new());
        let completed_class = self.gensym("completed_class");
        self.under_way.push(ClassParts { lexical_members, completed_class, needs_class_cell: false, methods: Vec::new(), attributes: Vec::new(), held: Vec::new(),
            ranking: Vec::new(), annotated_names: Vec::new(), uncertain: Vec::new(), arms: 0, cannot,
            book: None, book_tracked: HashSet::new() });
        // The body uses the metaclass's actual mapping from its first statement.
        if table.has_any("ext.stmt.class.builder") || self.class_body_names_locals(self.pos, on_one_line) {
            let made = self.class_book();
            setup.push(made);
        }
        if let Some(word)=table.single("ext.stmt.class.detail.qualified") {self.member_ranked(word);self.parts().attributes.push(word.to_string());self.parts().held.push(constant(Value::text(&full_name)));}
        // Seed documentation only when the body starts with a docstring.
        if let Some(word) = table.single("ext.stmt.class.detail.doc") {
            let said = self.tokens.get(self.pos).filter(|t| t.shape == Shape::Quote).map(|t| Value::text(&t.lexeme));
            if let Some(said) = said { self.member_ranked(word); self.parts().attributes.push(word.to_string()); self.parts().held.push(constant(said)); }
        }
        let book = self.parts().book.clone().expect("class namespace");
        let module_name = table.single("ext.system.module.name").unwrap_or_default();
        let module_value = self.read(module_name);
        let module_word = table.single("ext.stmt.class.detail.module").unwrap_or_default();
        let target = self.read_to_write(book.ident.as_ref());
        setup.push(prim_call(Prim::ClassWork(19), vec![target, constant(Value::text(module_word)), module_value]));
        let seed = self.class_names_map();
        // Copy the body metadata through the prepared mapping's real setter.
        let seeded = prim_call(Prim::ClassWork(15), vec![seed]);
        setup.push(Form::Write(book.clone(), Box::new(seeded)));
        while !matches!(self.look().shape, Shape::Finish | Shape::Close) {
            if on_one_line && self.on_stmt_end() { break; }
            if self.class_item(&mut setup)? { continue; }
            if on_one_line {
                while self.look().shape == Shape::Sign && table.spells("stmt.terminator", &self.look().lexeme) {
                    self.advance();
                }
            } else { self.skip_line_ends(); }
        }
        if !on_one_line {
            if self.look().shape != Shape::Close { return Err("Expected the end of a class body".into()); }
            self.advance();
        }
        if self.survey {
            let bound = self.parts().lexical_members.clone();
            self.surveyed.insert(body_source, ScopeWords { bound, ..ScopeWords::default() });
        }
        self.class_bindings.pop();
        self.class_globals.pop();
        self.class_met.pop();
        self.within = previous;
        self.named_before = named_outside;
        let parts = self.under_way.pop().expect("class body");
        if !parts.annotated_names.is_empty() && table.has_any("ext.stmt.class.annotations") {
            let row = parts.annotated_names.iter().flat_map(|(key, routine)| [key.clone(), routine.clone()]).collect();
            let book = parts.book.as_ref().expect("class namespace");
            let target = self.read_to_write(book.ident.as_ref());
            setup.push(prim_call(Prim::ClassWork(19), vec![target, constant(Value::text(crate::data::ANNOTATE_WORD)), prim_call(Prim::MakeArray, row)]));
        }
        if parts.cannot { setup.push(self.class_not_ready()); }
        setup.push(constant(Value::Nil));
        Ok((sequence(setup), parts.book.expect("class namespace").ident.to_string(), parts.needs_class_cell.then(|| parts.completed_class.ident.to_string()), parts.cannot))
    }

    fn class_with_receiver(&mut self) -> Res<(Form, bool)> {
        if self.table.has_any("ext.stmt.class.builder") { return self.python_class(); }
        self.advance();
        let class_title = self.original_words[self.pos].lexeme.clone();
        let named = self.need_word("as the class name")?;
        if self.on_any("ext.stmt.type_params.open") { self.class_type_parameters()?; }
        let table = self.table;
        let mut setup = Vec::new();
        let mut parent = None;
        let mut other_parents = Vec::new();
        let mut handed_words: Vec<(String, Form)> = Vec::new();
        // A class standing in a function is built the way any class is:
        // its members are written to addresses in the function's own
        // frame, and the class gathers them when the statement runs.
        let mut cannot = false;
        if table.single("ext.stmt.class.bases.open").map_or(false, |o| self.sign(o)) {
            self.advance();
            let end = table.single("ext.stmt.class.bases.close").ok_or("The bases need a closing mark")?;
            let mut levels = Vec::new();
            let mut spreading = false;
            for token in &self.tokens[self.pos..] {
                let word = token.lexeme.as_str();
                if levels.is_empty() && word == end { break; }
                if levels.is_empty() && (table.spells("op.mul", word) || table.spells("op.pow", word)) { spreading = true; }
                match word {
                    "(" => levels.push(")"), "[" => levels.push("]"), "{" => levels.push("}"),
                    _ if levels.last().copied() == Some(word) => { levels.pop(); }, _ => {}
                }
            }
            if spreading {
                let arguments = self.args("syntax.call.close", "syntax.call.separator")?;
                let kept = self.gensym("header_arguments");
                setup.push(Form::Write(kept.clone(), Box::new(prim_call(Prim::MakeTuple, arguments))));
                handed_words.push(("\0header".to_owned(), Form::Read(kept)));
            } else {
            let mut first = true;
            while !self.sign(end) {
                let expanded = table.spells("op.mul", &self.look().lexeme) || table.spells("op.pow", &self.look().lexeme);
                if expanded { self.advance(); cannot = true; }
                let keyword = self.look().shape == Shape::Bare && table.spells("stmt.assign", &self.glance(1).lexeme);
                // A keyword in the header goes, under a name no program
                // can spell, to the forebears' subclass hook; a table
                // without such a hook cannot run the form.
                let mut handed = None;
                // The keyword that names a metaclass says what builds
                // the class, and is handed over under a name of its own.
                let mut builder = false;
                if keyword {
                    let word = self.original_words[self.pos].lexeme.clone();
                    if table.has_any("ext.builtin.exceptions.syntax") && word == "__debug__" {
                        return Err(String::from("SyntaxError: cannot assign to __debug__"));
                    }
                    self.advance();
                    self.advance();
                    if table.spells("ext.stmt.class.metaclass", &word) { builder = true; }
                    else if table.has_any("ext.stmt.class.detail.subclass") { handed = Some(word); }
                    else { cannot = true; }
                }
                let value = self.expr(0)?;
                if builder {
                    let slot = self.gensym("metaclass");
                    setup.push(Form::Write(slot.clone(), Box::new(value)));
                    handed_words.push(("\0metaclass".to_owned(), Form::Read(slot)));
                } else if let Some(word) = handed {
                    let slot = self.gensym("handed");
                    setup.push(Form::Write(slot.clone(), Box::new(value)));
                    handed_words.push((format!("\0handed:{word}"), Form::Read(slot)));
                } else if !keyword && !expanded && (first || table.has_any("ext.stmt.class.detail.root")) {
                    let slot = self.gensym("parent");
                    setup.push(Form::Write(slot.clone(), Box::new(value)));
                    if first { parent = Some(slot); } else { other_parents.push(slot); }
                }
                first = false;
                match table.single("syntax.call.separator") {
                    Some(comma) if self.sign(comma) => { self.advance(); }
                    _ => break,
                }
            }
            self.need_sign(end, "after the bases")?;
            }
        }
        let full_name = self.full_name_of(&class_title);
        let previous = self.within.replace((class_title.clone(), parent.as_ref().map(|s| s.ident.to_string())));
        let named_outside = std::mem::replace(&mut self.named_before, self.naming.len());
        if table.has_any("ext.stmt.class.detail.root"){self.within=Some((full_name.clone(),parent.as_ref().map(|a|a.ident.to_string())));}
        self.need_intro()?;
        let on_one_line = !self.on_stmt_end() && self.look().shape != Shape::Open;
        if !on_one_line {
            self.skip_line_ends();
            if self.look().shape != Shape::Open { return Err("Expected an indented class body".to_string()); }
            self.advance();
            self.skip_line_ends();
        }
        let body_source = self.pos;
        let lexical_members = self.surveyed.get(&body_source).map(|scope| scope.bound.clone()).unwrap_or_default();
        self.class_bindings.push((self.layers.len(), HashMap::new()));
        self.class_globals.push((self.layers.len(), Vec::new()));
        self.class_met.push(Vec::new());
        let completed_class = self.gensym("completed_class");
        self.under_way.push(ClassParts { lexical_members, completed_class, needs_class_cell: false, methods: Vec::new(), attributes: Vec::new(), held: Vec::new(),
            ranking: Vec::new(), annotated_names: Vec::new(), uncertain: Vec::new(), arms: 0, cannot,
            book: None, book_tracked: HashSet::new() });
        // A body that spells `locals` or `vars` anywhere in it is
        // given its own namespace before its first statement runs, so
        // a write through either as the body's first statement finds
        // somewhere of its own already standing, not made as part of
        // the write.
        if self.class_body_names_locals(self.pos, on_one_line) {
            let made = self.class_book();
            setup.push(made);
        }
        if let Some(word)=table.single("ext.stmt.class.detail.qualified") {self.member_ranked(word);self.parts().attributes.push(word.to_string());self.parts().held.push(constant(Value::text(&full_name)));}
        // What the class says about itself is text standing alone at the
        // head of the body, kept under the word the table gives
        // (ext.stmt.class.detail.doc). A class that says nothing keeps
        // nothing under the word, rather than lacking the word.
        if let Some(word) = table.single("ext.stmt.class.detail.doc") {
            let said = self.tokens.get(self.pos).filter(|t| t.shape == Shape::Quote).map(|t| Value::text(&t.lexeme));
            self.member_ranked(word);
            self.parts().attributes.push(word.to_string());
            self.parts().held.push(constant(said.unwrap_or(Value::Nil)));
        }
        let before_body = setup.len();
        while !matches!(self.look().shape, Shape::Finish | Shape::Close) {
            if on_one_line && self.on_stmt_end() { break; }
            if self.class_item(&mut setup)? { continue; }
            if on_one_line {
                while self.look().shape == Shape::Sign && table.spells("stmt.terminator", &self.look().lexeme) {
                    self.advance();
                }
            } else { self.skip_line_ends(); }
        }
        if !on_one_line {
            if self.look().shape != Shape::Close { return Err("Expected the end of a class body".into()); }
            self.advance();
        }
        if self.survey {
            let bound = self.parts().lexical_members.clone();
            self.surveyed.insert(body_source, ScopeWords { bound, ..ScopeWords::default() });
        }
        self.class_bindings.pop();
        self.class_globals.pop();
        self.class_met.pop();
        self.within = previous;
        self.named_before = named_outside;
        let ClassParts { completed_class, needs_class_cell: _, methods, mut attributes, mut held, mut ranking, annotated_names, uncertain, cannot, book, book_tracked, .. } = self.under_way.pop().expect("the class body just read");
        if cannot {
            setup.truncate(before_body);
            setup.push(self.class_not_ready());
        }
        // The routines answering what the body annotated, carried under
        // a hidden name until the class's annotations are asked for; a
        // body that annotated nothing leaves the class without them.
        if table.has_any("ext.stmt.class.annotations") && !annotated_names.is_empty() {
            // Keys and routines alternate in one row, made as the body runs.
            ranking.push(crate::data::ANNOTATE_WORD.to_owned());
            attributes.push(crate::data::ANNOTATE_WORD.to_owned());
            let row: Vec<Form> = annotated_names.into_iter().flat_map(|(key, routine)| [key, routine]).collect();
            held.push(prim_call(Prim::MakeArray, row));
        }
        // The header's keywords ride along as entries under their hidden
        // names, for the building of the class to hand on.
        for (word, read) in handed_words {
            attributes.push(word);
            held.push(read);
        }
        // A member only an arm of a conditional writes to may stay
        // unwritten, and reading such a place outright would stop the
        // run. Each is glanced at instead, and each is emptied again
        // once the class holds what it found, so that a class statement
        // read a second time — one standing in a loop — does not find
        // what the pass before it left behind.
        let mut emptied = Vec::new();
        for (at, word) in attributes.iter().enumerate() {
            if !uncertain.iter().any(|n| n == word) { continue; }
            if let Form::Read(place) = held[at].clone() {
                held[at] = Form::Glance(place.clone());
                emptied.push(place);
            }
        }
        // A member the body's own live namespace governs -- one it
        // has mirrored a write of -- is settled by that namespace
        // alone from here on, its current pairs read in at the very
        // end: a `del`, of the name or through `locals()`, has already
        // left the class without it there, rather than the value its
        // place still happens to hold.
        let pushed_at: Vec<usize> = match &book {
            Some(_) => (0..attributes.len()).filter(|&i| !book_tracked.contains(&attributes[i])).collect(),
            None => (0..attributes.len()).collect(),
        };
        let pushed_attributes: Vec<String> = pushed_at.iter().map(|&i| attributes[i].clone()).collect();
        let pushed_held: Vec<Form> = pushed_at.into_iter().map(|i| held[i].clone()).collect();
        let mut values = Vec::new();
        if let Some(slot) = &parent { values.push(Form::Read(slot.clone())); }
        values.extend(other_parents.iter().cloned().map(Form::Read));
        values.extend(pushed_held);
        let has_book = book.is_some();
        if let Some(book_place) = &book { values.push(Form::Read(book_place.clone())); }
        let plan = Plan {
            name: class_title, answers: other_parents.len(), field_names: vec![], field_reach: vec![],
            shared_names: pushed_attributes, constant_names: vec![], methods, extends: parent.is_some(),
            ranking, has_book,
        };
        let declaration = Form::Class { plan: Rc::new(plan), values };
        setup.push(Form::Write(completed_class.clone(), Box::new(declaration)));
        let declaration = Form::Read(completed_class);
        if self.class_bindings.last().map_or(false, |(level, _)| *level == self.layers.len()) {
            let slot = self.gensym("inner_class");
            self.class_bindings.last_mut().expect("an outer class").1.insert(named, slot.clone());
            setup.push(Form::Write(slot, Box::new(declaration)));
        } else { setup.push(self.write(&named, declaration)); }
        for place in emptied { setup.push(Form::Forget(place)); }
        Ok((sequence(setup), cannot))
    }

    fn class_type_parameters(&mut self) -> Res<()> {
        let earlier = std::mem::replace(&mut self.forbids_await, true);
        self.advance();
        let table = self.table;
        let mut declared = Vec::new();
        loop {
            if ["ext.stmt.function.carries", "ext.stmt.function.carries.pairs"].iter().any(|key| self.on_any(key)) { self.advance(); }
            let parameter = self.need_word("among the type parameters")?;
            if table.has_any("ext.builtin.exceptions.syntax") && parameter == "__debug__" { return Err("SyntaxError: cannot assign to __debug__".to_owned()); }
            if declared.contains(&parameter) { return Err(table.single("ext.stmt.function.parameters.amiss").unwrap_or_default().into()); }
            declared.push(parameter);
            if self.on_any("ext.stmt.annotation") { self.advance(); self.expr_at(0, false)?; }
            if self.on_assign() { self.advance(); self.expr(0)?; }
            if self.on_any("ext.stmt.type_params.close") { break; }
            self.need_sign(table.single("syntax.call.separator").unwrap(), "between type parameters")?;
            if self.on_any("ext.stmt.type_params.close") { break; }
        }
        self.need_sign(table.single("ext.stmt.type_params.close").unwrap(), "after the type parameters")?;
        self.forbids_await = earlier;
        Ok(())
    }

    fn class_decl(&mut self) -> Res<Form> {
        if self.table.flag("ext.stmt.class.this.explicit") {
            return self.class_with_receiver().map(|(form, _)| form);
        }
        let table = self.table;
        let word = self.advance().lexeme;
        let name = self.need_word("as the class name")?;
        if table.has_any("ext.builtin.exceptions.syntax") && name == "__debug__" { return Err("SyntaxError: cannot assign to __debug__".to_owned()); }
        if self.on_any("ext.stmt.type_params.open") { self.class_type_parameters()?; }
        // A class of method names only may be built on several at once;
        // a class is built on one and answers to any number.
        let bare = table.spells("ext.stmt.class.interface", &word);
        let between = table.single("syntax.call.separator").map(str::to_string);
        let mut under = None;
        let mut answers: Vec<String> = Vec::new();
        if self.key("ext.stmt.class.extends") {
            self.advance();
            let first = self.need_word("as the class it is built on")?;
            match bare {
                true => answers.push(first),
                false => under = Some(first),
            }
            while between.as_ref().map_or(false, |s| self.sign(s)) {
                self.advance();
                answers.push(self.need_word("as another class named there")?);
            }
        }
        if self.key("ext.stmt.class.implements") {
            self.advance();
            answers.push(self.need_word("as the class it answers to")?);
            while between.as_ref().map_or(false, |s| self.sign(s)) {
                self.advance();
                answers.push(self.need_word("as another class named there")?);
            }
        }
        let outer = self.within.replace((name.clone(), under.clone()));
        // A class body may open on a line of its own, as any body may.
        self.skip_lead_word();
        self.skip_line_ends();
        let opens = table.strings("block.open");
        let k = opens.iter().position(|o| self.lexeme_of(o)).ok_or_else(|| format!("Expected '{}' to open the class, got '{}'", opens[0], self.look().lexeme))?;
        self.advance();
        let close = table.strings("block.close")[k].clone();
        let (fields, shared, constants) = (Vec::new(), Vec::new(), Vec::new());
        let reaches: Vec<Reach> = Vec::new();
        let methods = Vec::new();
        self.skip_line_ends();
        // What a method keeps between calls is set where the class is
        // declared, as a plain routine's own values are set where the
        // routine is defined.
        let statics_before = self.statics.len();
        let mut held = Members { fields, shared, constants, methods, reaches };
        self.class_members(&close, &mut held)?;
        let Members { fields, shared, constants, methods, reaches } = held;
        self.need_lexeme(&close)?;
        self.within = outer;
        // What it is built on first, then a value for each property, each
        // kept value and each constant, in the order the plan names them.
        let mut values = Vec::new();
        if let Some(under) = &under.clone() {
            let bound = self.class_binding(under);
            values.push(self.read_bound(under, &bound));
        }
        for named in &answers.clone() {
            let bound = self.class_binding(named);
            let read = self.read_bound(named, &bound);
            values.push(read);
        }
        let names = |parts: Vec<(String, Form)>, values: &mut Vec<Form>| -> Vec<String> {
            parts
                .into_iter()
                .map(|(member, value)| {
                    values.push(value);
                    member
                })
                .collect()
        };
        let field_names = names(fields, &mut values);
        let shared_names = names(shared, &mut values);
        let constant_names = names(constants, &mut values);
        let plan = Plan { name: name.clone(), answers: answers.len(), field_names, field_reach: reaches, shared_names, constant_names, methods, extends: under.is_some(), ranking: vec![], has_book: false };
        let made = Form::Class { plan: Rc::new(plan), values };
        let bound = self.class_binding(&name);
        let slot = self.global_address(&bound);
        let written = Form::Write(slot, Box::new(made));
        // What a method keeps between calls is set after the class is
        // bound, since what it is set to may name the class itself.
        let kept: Vec<Form> = self.statics.drain(statics_before..).collect();
        if kept.is_empty() {
            return Ok(written);
        }
        let mut items = vec![written];
        items.extend(kept);
        Ok(sequence(items))
    }

    /// The members a class or a bag declares, read one after another
    /// until the mark that closes the body. A bag's members are read
    /// again here, at the `use` that takes them in, so that they stand
    /// in the class taking them and not in the bag.
    fn class_members(&mut self, close: &str, held: &mut Members) -> Res<()> {
        let table = self.table;
        while !self.lexeme_of(close) && !self.exhausted() {
            let mut kept = false;
            let mut reach = Reach::Everywhere;
            while self.look().shape == Shape::Bare {
                if self.key("ext.stmt.class.shared") {
                    kept = true;
                } else if self.key("ext.stmt.class.hidden") {
                    reach = Reach::Alone;
                } else if self.key("ext.stmt.class.guarded") {
                    reach = Reach::Within;
                } else if !self.key("ext.stmt.class.modifier") {
                    break;
                }
                self.advance();
            }
            if self.key("ext.stmt.class.uses") {
                self.take_in_members(held)?;
            } else if self.key("ext.stmt.const") {
                self.advance();
                let member = self.need_word("as the constant name")?;
                self.need_assign("after the constant name")?;
                held.constants.push((member, self.expr(0)?));
            } else if self.key("stmt.function") {
                self.advance();
                let gives_cell = self.skip_reference();
                let member = self.need_word("as the method name")?;
                self.giving_cells.push(gives_cell);
                let program = self.method(&member);
                self.giving_cells.pop();
                held.methods.push((member, program?));
                // A parameter of the maker that names a property makes
                // the class carry that property too.
                for named in std::mem::take(&mut self.also_property) {
                    let bare = match table.letter("identifier.variable_prefix") {
                        Some(sigil) => named.trim_start_matches(sigil).to_string(),
                        None => named,
                    };
                    held.fields.push((bare, constant(Value::Nil)));
                    held.reaches.push(Reach::Everywhere);
                }
            } else {
                // A property, perhaps with a type word before its name.
                if self.look().shape == Shape::Bare && self.glance(1).shape == Shape::Bare {
                    self.advance();
                }
                // One declaration may name several properties, written
                // apart the way a call's arguments are.
                let apart = table.single("syntax.call.separator").map(str::to_string);
                loop {
                    let member = self.need_word("as the property name")?;
                    let bare = match table.letter("identifier.variable_prefix") {
                        Some(sigil) => member.trim_start_matches(sigil).to_string(),
                        None => member.clone(),
                    };
                    let value = match self.on_assign() {
                        true => {
                            self.advance();
                            self.expr(0)?
                        }
                        false => constant(Value::Nil),
                    };
                    if kept {
                        held.shared.push((bare, value));
                    } else {
                        held.fields.push((bare, value));
                        held.reaches.push(reach);
                    }
                    match &apart {
                        Some(sep) if self.sign(sep) => self.advance(),
                        _ => break,
                    };
                }
            }
            self.skip_line_ends();
        }
        Ok(())
    }

    /// A method: a program whose first parameter is the thing it is for,
    /// under the name the definition gives it (`$this`).
    fn method(&mut self, name: &str) -> Res<Rc<Routine>> {
        let deferred = self.pos.checked_sub(3).and_then(|at| self.tokens.get(at))
            .map_or(false, |word| self.table.spells("ext.stmt.async", &word.lexeme));
        if self.on_any("ext.stmt.type_params.open") { self.class_type_parameters()?; }
        let table = self.table;
        self.declared_at = (self.look().row as u32).saturating_sub(self.before);
        let open = table.single("syntax.call.open").ok_or_else(|| "This language has no call syntax".to_string())?;
        self.need_sign(open, "after method name")?;
        let explicit = table.flag("ext.stmt.class.this.explicit");
        let this = table.single("ext.stmt.class.this").unwrap_or_default().to_string();
        let mut params = Vec::new();
        if !explicit { params.push(this.clone()); }
        let (given, spares, also_property, said) = self.parameters(name, false)?;
        self.statics.extend(said);
        params.extend(given);
        // The thing it is for is always given, so every place moves by one.
        let least = params.len() - spares.len();
        let spares: Vec<(usize, usize)> = spares.into_iter().map(|(at, from)| (at + usize::from(!explicit), from)).collect();
        let formals = params.clone();
        if self.look().shape == Shape::Sign
            && (table.spells("stmt.function.returns", &self.look().lexeme) || table.spells("ext.stmt.function.returns", &self.look().lexeme))
        {
            self.advance();
            match table.has_any("ext.stmt.annotation") {
                true => { self.annotation_sites.push(("return".to_owned(), self.pos)); self.put_by_annotation(&["block.intro"])?; },
                false => {
                    self.skip_nothing_mark();
                    self.need_word("as a return type")?;
                }
            }
        }
        // A method may be named and not written out, in a class of
        // method names only; it answers with nothing. A body on the line
        // after the name is still a body.
        if self.on_stmt_end() && !self.block_opens_ahead() {
            let named = self.routine(name, Holds::Every, Traps::Yields, params, least, |_| Ok(constant(Value::Nil)))?;
            return match named {
                Form::Const(Value::Routine(p)) => Ok(p),
                _ => Err("A method must be a program".to_string()),
            };
        }
        let named = also_property.clone();
        self.also_property = also_property;
        let sigil = table.letter("identifier.variable_prefix");
        let enclosing_receiver = self.receiver.take();
        self.receiver = params.first().cloned();
        let local_defaults = spares.iter().map(|(place, _)| *place).collect();
        let program = self.routine(name, Holds::Every, Traps::Yields, params, least, |r| {
            r.layers.last_mut().unwrap().permits_async = deferred;
            r.generator_seen = deferred;
            let mut items = r.spare_values(spares, &formals)?;
            // What a parameter that names a property was handed is put
            // into the thing before the body runs.
            for member in &named {
                let bare = match sigil {
                    Some(mark) => member.trim_start_matches(mark).to_string(),
                    None => member.clone(),
                };
                let thing = r.read(&this);
                let held = r.read(member);
                items.push(prim_call(Prim::Onto, vec![thing, constant(Value::text(&bare)), held]));
            }
            // A body on the line of its own name is read the way any
            // body on the line of its head is read, so a method that
            // joins its statements with the mark that ends a statement
            // holds every one of them and not the first alone.
            let body = r.body()?;
            items.push(body);
            if explicit && table.blocks == Blocks::Indented { items.push(constant(Value::Nil)); }
            Ok(sequence(items))
        });
        self.receiver = enclosing_receiver;
        match program? {
            Form::Const(Value::Routine(mut p)) => {
                Rc::get_mut(&mut p).unwrap().local_defaults = local_defaults;
                Ok(p)
            },
            _ => Err("A method must be a program".to_string()),
        }
    }

    /// `foreach (a as v)` and `foreach (a as k => v)`: the array or map
    /// held aside and walked by position, its key and item bound at the
    /// head of each pass.
    fn foreach_stmt(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
        let open = table.single("syntax.group.open").ok_or_else(|| "A foreach needs syntax.group".to_string())?;
        let close = table.single("syntax.group.close").unwrap().to_string();
        self.need_sign(open, "after foreach")?;
        // A walk that hands out its items for writing walks the binding
        // itself, so that writing an item writes the array it came from.
        let mut named = match (self.look().shape, self.glance(1).shape) {
            (Shape::Bare, Shape::Bare) if table.spells("stmt.foreach.as", &self.glance(1).lexeme) => Some(self.advance().lexeme),
            _ => None,
        };
        // What is walked for its own cells need not be written out as a
        // name: a place in an array, a member, a class's own value or a
        // binding named as the run goes each keeps a cell too. Such a
        // subject is asked for its cell and a hidden name tied to it,
        // which stands in for the name the walk was not given.
        let mut ahead = Vec::new();
        let source = if named.is_some() {
            self.read(named.as_deref().expect("the name"))
        } else if self.mark_in_head(open, &close) {
            let read = self.expr(0)?;
            let cell = self.cell_of(read)?;
            self.gensyms += 1;
            let held = format!("#walked{}", self.gensyms);
            let to = self.address_to_write(&held);
            ahead.push(Form::Tie(to, Box::new(cell)));
            let stands = self.read(&held);
            named = Some(held);
            stands
        } else {
            self.expr(0)?
        };
        if !self.key("stmt.foreach.as") {
            return Err(format!("Expected '{}' in foreach, got '{}'", table.single("stmt.foreach.as").unwrap(), self.look().lexeme));
        }
        self.advance();
        let mut shares = table.single("ext.op.reference").map_or(false, |m| self.sign(m));
        // Whether the mark stood before the whole of the head and not
        // before the value alone, which tells a key marked as taking a
        // cell from a value so marked.
        let marked_first = shares;
        if shares {
            self.advance();
        }
        // A walk may hand its items to a place and not only to a name:
        // `foreach ($a as $b[0])`. Where the name is followed by more,
        // where it began is kept and read again at the head of every
        // pass, the item waiting in a cell of the walk's own.
        let first_at = self.pos;
        let first = self.need_word("as the foreach variable")?;
        let coupled = table.single("syntax.map.pair").map_or(false, |m| self.sign(m));
        let mut place: Option<usize> = None;
        let (key, mut item) = if coupled {
            // A key is no place: it is what a member is called, and a
            // name given it has no cell of the walk's to be tied to.
            if let (true, Some(said)) = (marked_first, table.single("ext.op.walk.key.no_cell")) {
                self.stopped_fatally = true;
                return Err(said.to_string());
            }
            self.advance();
            if table.single("ext.op.reference").map_or(false, |m| self.sign(m)) {
                self.advance();
                shares = true;
            }
            let began = self.pos;
            let held = self.need_word("as the foreach value")?;
            if !self.sign(&close) {
                place = Some(began);
            }
            (Some(first), held)
        } else {
            if !self.sign(&close) {
                place = Some(first_at);
            }
            (None, first)
        };
        if place.is_some() {
            if shares {
                return Err("A walk hands its items for writing to a name, not to a place".to_string());
            }
            self.step_to_close(&close)?;
            self.gensyms += 1;
            item = format!("#item{}", self.gensyms);
        }
        let shares = match (shares, &named) {
            (true, Some(name)) => Some(name.clone()),
            (true, None) => return Err("A foreach that hands out its items for writing needs a named array".to_string()),
            (false, _) => None,
        };
        self.need_sign(&close, "after the foreach names")?;
        ahead.push(self.walk(source, key, item, shares, place)?);
        Ok(sequence(ahead))
    }

    /// Whether the head of a walk, read from where its subject begins,
    /// carries the mark handing items out for writing. The subject is
    /// stepped over by counting the grouping marks, so a call written
    /// within it does not read as the end of the head.
    fn mark_in_head(&self, open: &str, close: &str) -> bool {
        let table = self.table;
        let mark = match table.single("ext.op.reference") {
            Some(mark) => mark,
            None => return false,
        };
        let mut at = self.pos;
        let mut deep = 0usize;
        let mut said_as = false;
        while at < self.tokens.len() {
            let w = &self.tokens[at];
            let a_sign = w.shape == Shape::Sign;
            if a_sign && w.lexeme == open {
                deep += 1;
            } else if a_sign && w.lexeme == close {
                if deep == 0 {
                    break;
                }
                deep -= 1;
            } else if !said_as {
                said_as = deep == 0 && w.shape == Shape::Bare && table.spells("stmt.foreach.as", &w.lexeme);
            } else if a_sign && w.lexeme == mark {
                return true;
            }
            at += 1;
        }
        false
    }

    /// Step to the mark closing a grouping, reading nothing on the way,
    /// so what stands within may be read later.
    fn step_to_close(&mut self, close: &str) -> Res<()> {
        let open = self.table.single("syntax.group.open").unwrap_or("(").to_string();
        let mut deep = 1usize;
        while deep > 0 {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            if self.sign(&open) {
                deep += 1;
            } else if self.sign(close) {
                deep -= 1;
                if deep == 0 {
                    break;
                }
            }
            self.advance();
        }
        Ok(())
    }

    /// The walk itself: a place counted to the extent, the key and the
    /// item read from it at the head of every pass.
    fn walk(&mut self, source: Form, key: Option<String>, item: String, shares: Option<String>, place: Option<usize>) -> Res<Form> {
        let bag = self.gensym("bag");
        let bag_name = bag.ident.to_string();
        // A walk over the copy takes the copy here; one handing out the
        // items' own cells walks the array itself, so what the body adds
        // or takes away lengthens or shortens the walk. Either way a
        // thing that is its own walk, or that hands another over to be
        // walked for it, is settled here and once: a thing is a handle,
        // so holding it again holds the same thing.
        let taken = match shares.is_some() {
            true => source,
            false => prim_call(Prim::Walked, vec![source]),
        };
        let hold = Form::Write(bag, Box::new(taken));
        let at = self.gensym("at");
        let at_name = at.ident.to_string();
        let start = Form::Write(at, Box::new(constant(Value::Small(0))));
        // A language may have the walk keep its place by the item it
        // handed out and not by counting. The cell of that item is tied
        // to a name of the walk's own, which holds nothing at all until
        // the first pass has handed one out.
        let by_hand = shares.is_some() && self.table.flag("ext.op.walk.live");
        let kept = by_hand.then(|| self.gensym("held"));
        let kept_name = kept.as_ref().map(|k| k.ident.to_string());
        let empty = kept.map(|k| Form::Write(k, Box::new(constant(Value::Nil))));
        if let Some(k) = &key {
            self.address_to_write(k);
        }
        self.address_to_write(&item);
        let over = shares.clone().unwrap_or_else(|| bag_name.clone());
        let alive = shares.is_some();
        // A thing that is its own walk keeps no cells of its own to hand
        // out, and a language with words for that says so.
        let alone = alive.then(|| {
            let walked = self.read(&over);
            prim_call(Prim::AloneWalk, vec![walked])
        });
        let (test_at, test_over) = (at_name.clone(), over.clone());
        let (walk, walk_at) = (over.clone(), at_name.clone());
        let (step_at, step_over) = (at_name, over);
        let (body_kept, step_kept) = (kept_name.clone(), kept_name.clone());
        // Only a language with things to take members off need ask
        // whether the place a pass has reached still holds one.
        let things = self.table.single("ext.stmt.class").is_some();
        let looped = self.cycle(
            move |r| {
                let (walking, here) = (r.read(&test_over), r.read(&test_at));
                Ok(prim_call(Prim::MoreYet, vec![walking, here]))
            },
            move |r| {
                let mut items = Vec::new();
                match &shares {
                    // The name is tied to the item's own cell.
                    Some(named) => {
                        let at = r.read(&walk_at);
                        // Reached the way the walk reads it, so that the
                        // item tied here and the array walked are the
                        // one cell and not two.
                        let held = r.address_to_read(named);
                        let tied = r.address_to_write(&item);
                        items.push(Form::Tie(tied, Box::new(Form::ShareItem(held, Box::new(at)))));
                        // The walk's own name takes the same cell: a
                        // place asked twice for its cell answers with
                        // the one it was made into the first time.
                        if let Some(mine) = &body_kept {
                            let at = r.read(&walk_at);
                            let held = r.address_to_read(named);
                            let ours = r.address_to_write(mine);
                            items.push(Form::Tie(ours, Box::new(Form::ShareItem(held, Box::new(at)))));
                        }
                    }
                    None => {
                        let (bag, at) = (r.read(&walk), r.read(&walk_at));
                        let found = prim_call(Prim::AtHand, vec![bag, at]);
                        items.push(r.write(&item, found));
                    }
                }
                // What is at hand is asked for before what it is named,
                // since a thing that is its own walk answers in that
                // order when it is asked both.
                if let Some(k) = key {
                    let (bag, at) = (r.read(&walk), r.read(&walk_at));
                    let found = prim_call(Prim::NamedHere, vec![bag, at]);
                    items.push(r.write(&k, found));
                }
                // Where the walk hands its items to a place, the place
                // is read again here, the item waiting in the walk's
                // own cell.
                if let Some(began) = place {
                    let after = r.pos;
                    let write = if r.table.single("ext.op.tuple").is_some() && r.table.single("ext.stmt.unpack").is_some() {
                        let ends = r.divided_at(began, r.tokens.len(), "stmt.for.in");
                        let end = *ends.first().ok_or_else(|| "Expected loop target".to_string())?;
                        let mut words = Vec::new();
                        if r.target_words(began..end, &mut words) {
                            for word in words { r.claim(&word); }
                        }
                        r.distribute(began..end, &item).map_err(|e| r.loop_target_error(began..end, e))?
                    } else {
                        r.pos = began;
                        let target = r.expr_at(0, false)?;
                        let was = r.waiting.replace(item.clone());
                        let stood = r.look().clone();
                        let done = r.write_into(target, false, None, stood);
                        r.waiting = was;
                        done?
                    };
                    r.pos = after;
                    items.push(write);
                }
                items.push(r.body()?);
                let pass = sequence(items);
                if !things {
                    return Ok(pass);
                }
                // A member taken off a thing mid-walk leaves its place
                // empty, so that the members past it stay where they
                // were. Such a place is passed over: the walk goes
                // straight on to the next.
                // A walk hands out only what it reaches: the class it
                // is written in, where it is written in one, says which.
                let here = match &r.within {
                    Some((named, _)) => constant(Value::text(named)),
                    None => constant(Value::Nil),
                };
                let (bag, at) = (r.read(&walk), r.read(&walk_at));
                let there = prim_call(Prim::Kept, vec![bag, at, here]);
                Ok(r.choose(there, pass, constant(Value::Nil)))
            },
            Some(move |r: &mut Self| {
                let next = match &step_kept {
                    // One past where the item handed out now lies: the
                    // body may have carried it further along the array,
                    // or taken it out from under the walk altogether.
                    Some(mine) => {
                        let (walking, here) = (r.read(&step_over), r.read(&step_at));
                        let ours = r.address_to_write(mine);
                        prim_call(Prim::PastHeld, vec![walking, here, Form::Share(ours)])
                    }
                    None => {
                        let here = r.read(&step_at);
                        prim_call(Prim::Plus, vec![here, constant(Value::Small(1))])
                    }
                };
                let counted = r.write(&step_at, next);
                let walking = r.read(&step_over);
                Ok(sequence(vec![counted, prim_call(Prim::StepOn, vec![walking])]))
            }),
        )?;
        // Once the walk is over it lets its last item go, so that the
        // place holding it counts as shared only while a name of the
        // program's own still holds it.
        let loosed = kept_name.map(|mine| {
            let ours = self.address_to_write(&mine);
            Form::Forget(ours)
        });
        let mut all = Vec::new();
        all.extend(alone);
        all.push(hold);
        all.push(start);
        all.extend(empty);
        all.push(looped);
        all.extend(loosed);
        Ok(sequence(all))
    }

    /// `for (init; test; step) body`: the init, then a cycle whose step
    /// runs after each pass, `continue` included. An empty test is true.
    fn three_part_for(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
        let open = table.single("syntax.group.open").ok_or_else(|| "A for loop needs syntax.group".to_string())?;
        let close = table.single("syntax.group.close").unwrap();
        let end = table.single("stmt.terminator").ok_or_else(|| "A for loop needs stmt.terminator".to_string())?.to_string();
        self.need_sign(open, "after for")?;
        let init = self.clauses(&end)?;
        self.need_sign(&end, "after the for loop's start")?;
        let test = if self.sign(&end) { constant(Value::Flag(true)) } else { self.expr(0)? };
        self.need_sign(&end, "after the for loop's test")?;
        let step = self.clauses(close)?;
        self.need_sign(close, "after the for clauses")?;
        let body = self.body_limb(Traps::Naught)?;
        let looped = Form::Cycle { test: Box::new(test), body: Box::new(body), step: Some(Box::new(step)), after: false, otherwise: None };
        Ok(sequence(vec![init, looped]))
    }

    fn begins_match(&self) -> bool {
        let table = self.table;
        if !self.key("ext.stmt.match") { return false; }
        let mut closing = Vec::new();
        for token in &self.tokens[self.pos + 1..] {
            if matches!(token.shape, Shape::Finish | Shape::LineEnd) { return false; }
            if token.shape != Shape::Sign { continue; }
            if closing.is_empty() {
                if table.spells("block.intro", &token.lexeme) { return true; }
                if table.spells("stmt.assign", &token.lexeme) { return false; }
            }
            if closing.last().map_or(false, |end: &&str| *end == token.lexeme) {
                closing.pop();
            } else {
                for (open, close) in [("syntax.group.open", "syntax.group.close"), ("syntax.array.open", "syntax.array.close"), ("syntax.map.open", "syntax.map.close")] {
                    if table.spells(open, &token.lexeme) { closing.push(table.single(close).unwrap_or_default()); break; }
                }
            }
        }
        false
    }

    fn bad_case(&self) -> String {
        self.table.single("ext.stmt.match.invalid").unwrap_or("Invalid pattern").to_owned()
    }

    fn match_stmt(&mut self) -> Res<Form> {
        self.advance();
        let (value, tuple) = self.subject_of_match()?;
        let held = self.gensym("matched");
        let save = Form::Write(held.clone(), Box::new(value));
        if !self.on_any("block.intro") {
            if self.table.has_any("ext.builtin.exceptions.syntax") && self.look().shape == Shape::Bare && self.glance(1).lexeme == ":" {
                return Err(String::from("SyntaxError: invalid syntax"));
            }
            return Err(self.bad_case());
        }
        self.advance();
        self.skip_line_ends();
        if self.look().shape != Shape::Open { return Err(self.bad_case()); }
        self.advance();
        let mut arms = Vec::new();
        loop {
            self.skip_line_ends();
            if self.look().shape == Shape::Close { self.advance(); break; }
            if !self.key("ext.stmt.match.case") {
                if self.table.has_any("ext.builtin.exceptions.syntax") && self.look().shape == Shape::Bare && self.glance(1).lexeme == "=" {
                    return Err(String::from("SyntaxError: invalid syntax"));
                }
                return Err(self.bad_case());
            }
            self.advance();
            self.pattern_kinds.clear();
            let starts_wide = self.on_any("op.mul");
            let first = if starts_wide { self.advance(); self.pattern_name()? } else { self.pattern_choice()? };
            let mut members = vec![first];
            let mut spread = if starts_wide { Some(0) } else { None };
            let mut separated = false;
            while self.on_any("syntax.call.separator") {
                separated = true;
                self.advance();
                if self.on_any("block.intro") || self.key("ext.stmt.match.guard") { break; }
                if self.on_any("op.mul") {
                    if spread.is_some() { return Err(self.bad_case()); }
                    spread = Some(members.len());
                    self.advance();
                    members.push(self.pattern_name()?);
                } else { members.push(self.pattern_choice()?); }
            }
            if starts_wide && !separated { return Err(self.bad_case()); }
            let pattern = if separated { crate::form::CaseTest::Series { members, spread } } else { members.pop().unwrap() };
            let names = pattern.names().map_err(|_| self.bad_case())?;
            let slots = names.into_iter().map(|name| {
                let address = self.address_to_write(&name);
                (name, address)
            }).collect();
            let kinds = std::mem::take(&mut self.pattern_kinds);
            let mut fits = Form::Fits { value: Box::new(Form::Read(held.clone())), test: Rc::new(pattern), slots, tuple, kinds };
            if self.key("ext.stmt.match.guard") {
                self.advance();
                let guard = self.expr(0)?;
                fits = self.choose(fits, guard, constant(Value::Flag(false)));
            }
            if !self.on_any("block.intro") { return Err(self.bad_case()); }
            let same_line = self.glance(1).row == self.look().row;
            let mut statements = vec![self.body()?];
            if same_line {
                while self.look().shape == Shape::Sign && self.on_stmt_end() {
                    self.advance();
                    if matches!(self.look().shape, Shape::Finish | Shape::Close | Shape::LineEnd) { break; }
                    statements.push(self.stmt()?);
                }
            }
            arms.push((fits, sequence(statements)));
        }
        if arms.is_empty() { return Err(self.bad_case()); }
        let mut tail = constant(Value::Nil);
        for (test, body) in arms.into_iter().rev() { tail = self.choose(test, body, tail); }
        Ok(sequence(vec![save, tail]))
    }

    fn subject_of_match(&mut self) -> Res<(Form, bool)> {
        let table = self.table;
        let mut grouped = false;
        if self.on_any("syntax.group.open") {
            let mut nesting = Vec::new();
            for token in self.tokens.iter().skip(self.pos + 1) {
                if token.shape != Shape::Sign { continue; }
                if nesting.is_empty() {
                    if table.spells("syntax.call.separator", &token.lexeme) { grouped = true; }
                    if table.spells("syntax.group.close", &token.lexeme) { break; }
                }
                if nesting.last().map_or(false, |end: &&str| *end == token.lexeme) { nesting.pop(); }
                else {
                    for stem in ["syntax.group", "syntax.array", "syntax.map"] {
                        if table.spells(&format!("{}.open", stem), &token.lexeme) {
                            nesting.push(table.single(&format!("{}.close", stem)).unwrap_or_default());
                            break;
                        }
                    }
                }
            }
            grouped |= table.spells("syntax.group.close", &self.glance(1).lexeme);
        }
        if grouped { self.advance(); }
        let mut parts = Vec::new();
        let mut comma = false;
        if !(grouped && self.on_any("syntax.group.close")) {
            parts.push(self.expr(0)?);
            while self.on_any("syntax.call.separator") {
                comma = true;
                self.advance();
                if self.on_any("block.intro") || (grouped && self.on_any("syntax.group.close")) { break; }
                parts.push(self.expr(0)?);
            }
        }
        if grouped { self.need_sign(table.single("syntax.group.close").unwrap_or_default(), "after the subject")?; }
        let tuple = grouped || comma;
        let value = if tuple { prim_call(Prim::MakeArray, parts) } else { parts.pop().unwrap() };
        Ok((value, tuple))
    }

    fn pattern_name(&mut self) -> Res<crate::form::CaseTest> {
        use crate::form::CaseTest;
        let word = self.need_word("as a pattern binding")?;
        if self.table.has_any("ext.builtin.exceptions.syntax") && word == "__debug__" {
            return Err(String::from("SyntaxError: cannot assign to __debug__"));
        }
        if self.table.spells("ext.stmt.match.wildcard", &word) { return Ok(CaseTest::Ignore); }
        if ["literal.true", "literal.false", "literal.null"].iter().any(|label| self.table.spells(label, &word)) { return Err(self.bad_case()); }
        Ok(CaseTest::Keep(word))
    }

    fn pattern_choice(&mut self) -> Res<crate::form::CaseTest> {
        use crate::form::CaseTest;
        let first = self.pattern_single()?;
        let mut test = if self.on_any("ext.stmt.match.or") {
            let mut alternatives = vec![first];
            loop {
                self.advance();
                alternatives.push(self.pattern_single()?);
                if !self.on_any("ext.stmt.match.or") { break; }
            }
            CaseTest::AnyOf(alternatives)
        } else { first };
        if self.key("ext.stmt.match.as") {
            self.advance();
            let opening = self.pos;
            if self.key("ext.stmt.match.wildcard") {
                self.pos = opening;
                return Err(String::from("SyntaxError: cannot use '_' as a target"));
            }
            let name = match self.pattern_name() {
                Ok(CaseTest::Keep(name)) => name,
                _ => { self.pos = opening; return Err(self.pattern_target_wrong()); }
            };
            // A binding here is a bare name alone; a dot, a call or an
            // index after the name makes it a member, call or index.
            if self.on_any("op.pipe") {
                let (end, row) = self.pattern_chain_end(opening);
                self.range_end = Some((end, row));
                self.pos = opening;
                return Err(String::from("SyntaxError: cannot use attribute as pattern target"));
            }
            if self.on_any("syntax.call.open") {
                return Err(String::from("SyntaxError: cannot use function call as pattern target"));
            }
            if self.on_any("syntax.array.open") {
                return Err(String::from("SyntaxError: cannot use subscript as pattern target"));
            }
            test = CaseTest::Also { test: Box::new(test), name };
        }
        Ok(test)
    }

    /// The reference's refusal of an `as` binding that is not a plain
    /// name: the kind of thing written there, named in the message.
    fn pattern_target_wrong(&self) -> String {
        if self.on_any("syntax.group.open") {
            if let Some(close) = self.pair_close(self.pos, self.tokens.len()) {
                if self.pos + 1 == close || self.pair_holds(self.pos, close, &[","]) {
                    return String::from("SyntaxError: cannot use tuple as pattern target");
                }
                return String::from("SyntaxError: cannot use expression as pattern target");
            }
        }
        if self.on_any("syntax.array.open") {
            return String::from("SyntaxError: cannot use list as pattern target");
        }
        String::from("SyntaxError: cannot use expression as pattern target")
    }

    /// The column and row just past the dotted name that begins at
    /// `at`, a name and its members run name . name . name ... as far
    /// as they go.
    fn pattern_chain_end(&self, at: usize) -> (usize, u32) {
        let mut i = at;
        while i + 2 < self.tokens.len()
            && self.tokens[i].shape == Shape::Bare
            && self.tokens[i + 1].shape == Shape::Sign && self.table.spells("op.pipe", &self.tokens[i + 1].lexeme)
            && self.tokens[i + 2].shape == Shape::Bare {
            i += 2;
        }
        let last = &self.tokens[i];
        let end = if last.end_column > 0 { last.end_column } else { last.column + last.lexeme.chars().count() };
        (end, last.end_row.max(last.row))
    }

    fn pattern_single(&mut self) -> Res<crate::form::CaseTest> {
        use crate::form::CaseTest;
        let table = self.table;
        if self.look().shape == Shape::Numeral || self.on_any("op.sub") {
            let negative = self.on_any("op.sub");
            if negative { self.advance(); }
            if self.look().shape != Shape::Numeral { return Err(self.bad_case()); }
            let text = self.advance().lexeme;
            let mut number = numeral(&text, table)?;
            if negative {
                number = math::compute(math::Calc::Minus, &Value::Small(0), &number).ok_or_else(|| self.bad_case())??;
            }
            return Ok(CaseTest::Equal(number));
        }
        if matches!(self.look().shape, Shape::Quote | Shape::CharacterRow) {
            let mut numbers = Vec::new();
            loop {
                match self.look().shape {
                    Shape::Quote => numbers.extend(self.advance().lexeme.chars().map(|ch| ch as u32)),
                    Shape::CharacterRow => numbers.extend(self.advance().lexeme.split_whitespace().map(|word| word.parse::<u32>().unwrap())),
                    _ => break,
                }
            }
            return Ok(CaseTest::Equal(Value::characters(numbers)));
        }
        if self.on_any("syntax.array.open") || self.on_any("syntax.group.open") {
            let array = self.on_any("syntax.array.open");
            let end = if array { "syntax.array.close" } else { "syntax.group.close" };
            self.advance();
            let mut members = Vec::new();
            let mut spread = None;
            let mut comma = false;
            while !self.on_any(end) {
                let item = if self.on_any("op.mul") {
                    self.advance();
                    if spread.replace(members.len()).is_some() { return Err(self.bad_case()); }
                    self.pattern_name()?
                } else { self.pattern_choice()? };
                members.push(item);
                if !self.on_any("syntax.call.separator") { break; }
                self.advance();
                comma = true;
            }
            self.need_sign(table.single(end).unwrap_or_default(), "after the pattern")?;
            if !array && !comma && members.len() == 1 {
                if spread.is_some() { return Err(self.bad_case()); }
                return Ok(members.pop().unwrap());
            }
            return Ok(CaseTest::Series { members, spread });
        }
        if self.on_any("syntax.map.open") {
            self.advance();
            let mut pairs = Vec::new();
            let mut rest: Option<(usize, String)> = None;
            while !self.on_any("syntax.map.close") {
                if let Some((start, _)) = &rest {
                    self.pos = *start;
                    let words = if table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: double star pattern must be the last (right-most) subpattern in the mapping pattern".to_owned() } else { self.bad_case() };
                    return Err(words);
                }
                if self.on_any("op.pow") {
                    self.advance();
                    let start = self.pos;
                    if table.has_any("ext.builtin.exceptions.syntax") && self.look().lexeme == "_" {
                        return Err(String::from("SyntaxError: invalid syntax"));
                    }
                    let CaseTest::Keep(name) = self.pattern_name()? else { return Err(self.bad_case()) };
                    rest = Some((start, name));
                } else {
                    let key = self.pattern_single()?;
                    if !matches!(key, CaseTest::Equal(_) | CaseTest::Worth(_)) { return Err(self.bad_case()); }
                    self.need_sign(table.single("syntax.map.pair").unwrap_or_default(), "between a key and its pattern")?;
                    pairs.push((key, self.pattern_choice()?));
                }
                if !self.on_any("syntax.map.separator") { break; }
                self.advance();
            }
            self.need_sign(table.single("syntax.map.close").unwrap_or_default(), "after the mapping pattern")?;
            return Ok(CaseTest::Table { pairs, rest: rest.map(|(_, name)| name) });
        }
        if self.look().shape != Shape::Bare { return Err(self.bad_case()); }
        for (label, value) in [("literal.null", Value::Nil), ("literal.true", Value::Flag(true)), ("literal.false", Value::Flag(false))] {
            if self.key(label) { self.advance(); return Ok(CaseTest::Equal(value)); }
        }
        let binding = self.pattern_name()?;
        let mut path = Vec::new();
        while self.on_any("op.pipe") {
            self.advance();
            path.push(self.need_word("after the member mark")?);
        }
        let opens_call = self.on_any("syntax.call.open");
        if path.is_empty() && !opens_call { return Ok(binding); }
        // A dotted name, or the class ahead of a class pattern's brackets,
        // is worked out ahead of the subject and handed to the fitting by
        // its place among such values.
        let CaseTest::Keep(head) = &binding else { return Err(self.bad_case()) };
        let mut worked = self.read(head);
        for member in &path { worked = prim_call(Prim::Of, vec![worked, constant(Value::text(member))]); }
        let place = self.pattern_kinds.len();
        self.pattern_kinds.push(worked);
        if !opens_call { return Ok(CaseTest::Worth(place)); }
        self.advance();
        let mut positional = Vec::new();
        let mut named: Vec<(String, CaseTest)> = Vec::new();
        while !self.on_any("syntax.call.close") {
            if self.look().shape == Shape::Bare && table.spells("stmt.assign", &self.glance(1).lexeme) {
                let word = self.advance().lexeme;
                if table.has_any("ext.builtin.exceptions.syntax") && word == "__debug__" { return Err(String::from("SyntaxError: cannot assign to __debug__")); }
                if named.iter().any(|(seen, _)| *seen == word) {
                    return Err(if table.has_any("ext.builtin.exceptions.syntax") { format!("SyntaxError: attribute name repeated in class pattern: {word}") } else { self.bad_case() });
                }
                self.advance();
                named.push((word, self.pattern_choice()?));
            } else if !named.is_empty() {
                return Err(if table.has_any("ext.builtin.exceptions.syntax") { String::from("SyntaxError: positional patterns follow keyword patterns") } else { self.bad_case() });
            } else {
                positional.push(self.pattern_choice()?);
            }
            if !self.on_any("syntax.call.separator") { break; }
            self.advance();
        }
        self.need_sign(table.single("syntax.call.close").unwrap_or_default(), "after the class pattern")?;
        Ok(CaseTest::Shape { kind: place, positional, named })
    }

    /// `switch (v) { case a: ... default: ... }`. The first case equal to
    /// the value gives a starting section (the default's, wherever it
    /// stands, when none is); every section from there on runs, since a
    /// section falls into the next unless it breaks. The whole is a
    /// one-pass cycle so that `break` and `continue` leave it.
    fn switch_stmt(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
        let open = table.single("syntax.group.open").ok_or_else(|| "A switch needs syntax.group".to_string())?;
        self.need_sign(open, "after switch")?;
        let subject = self.expr(0)?;
        self.need_sign(table.single("syntax.group.close").unwrap(), "after the switch value")?;
        let held = self.gensym("switch");
        let held_name = held.ident.to_string();
        let keep = Form::Write(held, Box::new(subject));
        let start = self.gensym("start");
        let start_name = start.ident.to_string();
        self.skip_lead_word();
        self.skip_line_ends();
        let opens = table.strings("block.open");
        // A switch opens with a bracket or, where a language spells one,
        // with the mark standing for a bracket; the words that close
        // such a block close this one.
        let closers = match opens.iter().position(|o| self.lexeme_of(o)) {
            Some(k) => {
                self.advance();
                vec![table.strings("block.close")[k].clone()]
            }
            None if table.single("ext.stmt.block.instead").map_or(false, |m| self.sign(m)) => {
                self.advance();
                table.strings("ext.stmt.block.instead.close").to_vec()
            }
            None => return Err(format!("Expected '{}' to open the switch, got '{}'", opens[0], self.look().lexeme)),
        };
        let mut stops = closers.clone();
        stops.extend(table.strings("ext.stmt.case").iter().cloned());
        stops.extend(table.strings("ext.stmt.default").iter().cloned());
        // (test for the section, or None for the default; its body)
        let mut sections: Vec<(Option<Form>, Form)> = Vec::new();
        self.skip_line_ends();
        while !closers.iter().any(|w| self.lexeme_of(w)) && !self.exhausted() {
            let word = self.need_word("as case or default")?;
            let is_case = table.spells("ext.stmt.case", &word);
            if !is_case && !table.spells("ext.stmt.default", &word) {
                return Err(format!("Expected a case in the switch, got '{}'", word));
            }
            let test = if is_case {
                let value = self.expr(0)?;
                let held = self.read(&held_name);
                Some(prim_call(Prim::Eq, vec![held, value]))
            } else {
                None
            };
            if !(self.on_any("ext.stmt.case.mark") || self.on_stmt_end()) {
                return Err(format!("Expected '{}' after the case, got '{}'", table.single("ext.stmt.case.mark").unwrap(), self.look().lexeme));
            }
            // Closing a case the way a statement is closed still reads,
            // and where the language has words for it the reader asks
            // for the other mark instead.
            if !self.on_any("ext.stmt.case.mark") {
                if let Some(instead) = table.single("ext.stmt.case.mark.instead") {
                    let row = (self.look().row as u32).saturating_sub(self.before);
                    let told = Form::Remark("deprecated", Rc::from(instead), row);
                    self.noted_when_read.push(told);
                }
            }
            self.advance();
            let body = self.limb(Traps::Naught, |r| r.stmts_until(&stops))?;
            sections.push((test, body));
        }
        if !closers.iter().any(|w| self.lexeme_of(w)) {
            return Err(format!("Expected '{}' to close the switch, got '{}'", closers[0], self.look().lexeme));
        }
        self.advance();
        // start = the first section whose test holds, else the default's
        // index, else one past the end.
        let none = sections.len() as i64;
        let fallback = sections.iter().position(|(t, _)| t.is_none()).map_or(none, |i| i as i64);
        let mut choice = constant(Value::Small(fallback));
        let mut bodies = Vec::new();
        for (i, (test, body)) in sections.into_iter().enumerate().rev() {
            if let Some(test) = test {
                choice = self.choose(test, constant(Value::Small(i as i64)), choice);
            }
            bodies.push((i, body));
        }
        bodies.reverse();
        let mut run = Vec::new();
        for (i, body) in bodies {
            let from = self.read(&start_name);
            let reached = prim_call(Prim::Le, vec![from, constant(Value::Small(i as i64))]);
            let skip = constant(Value::Nil);
            run.push(self.choose(reached, body, skip));
        }
        let pass = Form::Cycle { test: Box::new(constant(Value::Flag(true))), body: Box::new(sequence(run)), step: None, after: true, otherwise: None };
        Ok(sequence(vec![keep, Form::Write(start, Box::new(choice)), pass]))
    }

    /// A statement without a keyword: a step, a bare call, an
    /// assignment or an expression.
    fn plain_stmt(&mut self) -> Res<Form> {
        let (here, next) = (self.look().clone(), self.glance(1).clone());
        if self.table.flag("ext.syntax.call.bare") && here.shape == Shape::Bare {
            // A builtin with no bracket after it; echo always, since a
            // bracket after echo opens a group, not its arguments.
            let op = self.table.prims.get(&here.lexeme).copied();
            let bracketed = self.table.single("syntax.call.open").map_or(false, |o| next.shape == Shape::Sign && next.lexeme == o);
            let bare = match op {
                Some(Prim::Tell) => true,
                // A writer written as an operator takes the whole of
                // what follows however it is written, so brackets after
                // it group rather than hold what it is given.
                Some(Prim::Out) if self.table.flag("ext.builtin.write.operator") => true,
                Some(Prim::Append | Prim::Replace | Prim::Front | Prim::Define | Prim::Gather | Prim::Erase | Prim::Standing | Prim::Hollow) | None => false,
                Some(_) => !bracketed,
            };
            if bare {
                return self.unbracketed_call(&here.lexeme);
            }
        }
        self.write_or_expr()
    }

    /// The number after break or continue, when the definition allows
    /// one (ext.stmt.break.levels): how many loops to leave.
    fn loop_levels(&mut self) -> Res<Vec<Form>> {
        if self.table.flag("ext.stmt.break.levels") && self.look().shape == Shape::Numeral {
            let n = self.advance().lexeme;
            let count = n.parse::<i64>().ok().filter(|n| *n >= 1).ok_or_else(|| format!("'{}' is not a number of loops to leave", n))?;
            return Ok(vec![constant(Value::Small(count))]);
        }
        Ok(Vec::new())
    }

    /// +1 for an increment sign, -1 for a decrement sign (ext.op.*).
    fn step_by(&self, t: &Token) -> Option<i64> {
        if t.shape != Shape::Sign {
            return None;
        }
        if self.table.spells("ext.op.increment", &t.lexeme) {
            Some(1)
        } else if self.table.spells("ext.op.decrement", &t.lexeme) {
            Some(-1)
        } else {
            None
        }
    }

    /// A builtin at the head of a statement, its arguments running
    /// unbracketed to the end of the statement (ext.syntax.call.bare).
    fn unbracketed_call(&mut self, name: &str) -> Res<Form> {
        self.advance();
        let sep = self.table.single("syntax.call.separator").map(str::to_string);
        let mut args = Vec::new();
        while !self.on_stmt_end() && !self.exhausted() {
            args.push(self.expr(0)?);
            match &sep {
                Some(s) if self.sign(s) => self.advance(),
                _ => break,
            };
        }
        self.named_call(name, args)
    }

    fn if_stmt(&mut self) -> Res<Form> {
        let keyword = self.table.blocks == Blocks::Worded;
        self.advance();
        let test = self.expr(0)?;
        let then = if keyword {
            self.skip_lead_word();
            let mut stops = self.table.strings("block.close").to_vec();
            stops.extend(self.table.strings("stmt.elif").iter().cloned());
            stops.extend(self.table.strings("stmt.else").iter().cloned());
            self.limb(Traps::Naught, |r| r.stmts_until(&stops))?
        } else {
            self.body_limb(Traps::Naught)?
        };
        let mut ahead = 0;
        while self.glance(ahead).shape == Shape::LineEnd || (self.glance(ahead).shape == Shape::Sign && self.table.separates(&self.glance(ahead).lexeme)) {
            ahead += 1;
        }
        let next = self.glance(ahead);
        let elif = next.shape == Shape::Bare && self.table.spells("stmt.elif", &next.lexeme);
        let else_ = next.shape == Shape::Bare && self.table.spells("stmt.else", &next.lexeme);
        let otherwise = if elif {
            self.pos += ahead;
            self.limb(Traps::Naught, |r| r.if_stmt())?
        } else if else_ {
            self.pos += ahead;
            self.advance();
            if self.key("stmt.if") {
                self.limb(Traps::Naught, |r| r.if_stmt())?
            } else if keyword {
                let closers = self.table.strings("block.close").to_vec();
                let arm = self.limb(Traps::Naught, |r| r.stmts_until(&closers))?;
                self.need_closer()?;
                arm
            } else {
                let arm = self.body_limb(Traps::Naught)?;
                let mut later = 0;
                while self.glance(later).shape == Shape::LineEnd { later += 1; }
                let token = self.glance(later);
                if self.table.has_any("ext.builtin.exceptions.syntax") && token.shape == Shape::Bare
                    && self.table.spells("stmt.elif", &token.lexeme) {
                    return Err(String::from("SyntaxError: 'elif' block follows an 'else' block"));
                }
                arm
            }
        } else {
            if keyword {
                self.need_closer()?;
            }
            self.limb(Traps::Naught, |_| Ok(constant(Value::Nil)))?
        };
        Ok(self.choose(test, then, otherwise))
    }

    fn type_names(&mut self) -> Res<bool> {
        if !self.table.flag("ext.stmt.type_parameters") || !self.on_any("op.index.open") { return Ok(false); }
        let earlier = std::mem::replace(&mut self.forbids_await, true);
        self.advance();
        let mut written: Vec<String> = Vec::new();
        loop {
            if self.on_any("ext.stmt.function.carries") || self.on_any("ext.stmt.function.carries.pairs") { self.advance(); }
            let parameter = self.look().lexeme.clone();
            self.need_word("among type parameters")?;
            written.push(parameter);
            if self.on_any("ext.stmt.annotation") {
                self.advance();
                self.put_by_annotation(&["stmt.assign", "ext.op.tuple", "op.index.close"])?;
            }
            if self.on_assign() {
                self.advance();
                self.put_by_annotation(&["ext.op.tuple", "op.index.close"])?;
            }
            if !self.on_any("ext.op.tuple") { break; }
            self.advance();
            if self.on_any("op.index.close") { break; }
        }
        self.need_sign(self.table.single("op.index.close").unwrap(), "after the type names")?;
        self.forbids_await = earlier;
        self.pending_types = written;
        Ok(true)
    }

    fn span_since(&self, beginning: usize) -> (u32, u32, u32, u32) {
        let left = &self.tokens[beginning];
        let right = &self.tokens[self.pos.max(beginning + 1) - 1];
        (left.row.saturating_sub(self.before), left.column.saturating_sub(1) as u32, if right.end_row == 0 { right.row.saturating_sub(self.before) } else { right.end_row.saturating_sub(self.before) },
            if right.end_column == 0 { (right.column + right.lexeme.len()).saturating_sub(1) as u32 } else { right.end_column.saturating_sub(1) as u32 })
    }

    fn located(&self, bounds: (u32, u32, u32, u32), expression: Form) -> Form {
        if self.table.has_any("ext.builtin.exceptions.traceback") { Form::Located(bounds, Box::new(expression)) }
        else { expression }
    }

    fn for_stmt(&mut self) -> Res<Form> {
        let table = self.table;
        let asynchronous = std::mem::take(&mut self.asynchronous);
        self.advance();
        if table.has_any("ext.builtin.exceptions.syntax") && self.glance(1).lexeme == "im" {
            return Err(String::from("SyntaxError: invalid syntax. Did you mean 'in'?"));
        }
        if table.has_any("ext.builtin.exceptions.syntax") {
            let limit = self.divided_at(self.pos, self.tokens.len(), "stmt.for.in").first().copied()
                .or_else(|| self.divided_at(self.pos, self.tokens.len(), "block.intro").first().copied());
            if limit.is_some_and(|end| self.has_added_target(self.pos..end)) {
                return Err(String::from("SyntaxError: cannot assign to expression"));
            }
        }
        if table.has_any("ext.builtin.exceptions.syntax") && matches!(self.look().shape, Shape::Quote | Shape::ByteQuote | Shape::Numeral) { return Err(String::from("SyntaxError: cannot assign to literal")); }
        let place = if table.single("ext.op.tuple").is_some() && table.single("ext.stmt.unpack").is_some() {
            self.divided_at(self.pos, self.tokens.len(), "stmt.for.in").first().copied().and_then(|end| {
                let start = self.pos;
                if start + 1 == end && self.tokens[start].shape == Shape::Bare { None }
                else { self.pos = end; Some(start) }
            })
        } else { None };
        let var = match place {
            Some(_) => self.gensym("each").ident.to_string(),
            None => self.need_word("as the loop variable")?,
        };
        if !self.key("stmt.for.in") {
            return Err(format!("Expected '{}' after for loop variable, got: {}", table.single("stmt.for.in").unwrap_or("in"), self.look().lexeme));
        }
        self.advance();
        // Let value-producing ranges validate their arguments before walking.
        let ranged = !table.flag("ext.builtin.range.value") && self.look().shape == Shape::Bare
            && table.prims.get(&self.look().lexeme) == Some(&Prim::Span)
            && table.single("syntax.call.open").map_or(false, |o| self.glance(1).shape == Shape::Sign && self.glance(1).lexeme == o);
        let (start, end) = if ranged && place.is_none() {
            self.advance();
            self.advance();
            let start = self.expr(0)?;
            if table.flag("ext.stmt.loop.else") && self.on_any("syntax.call.close") {
                self.advance();
                self.claim(&var);
                self.address_to_write(&var);
                return self.count_loop(&var, constant(Value::Small(0)), start, |r| r.body());
            }
            if let Some(sep) = table.single("syntax.call.separator") {
                self.need_sign(sep, "between the range bounds")?;
            }
            let end = self.expr(0)?;
            if table.single("ext.op.tuple").is_some() && self.on_any("syntax.call.separator") { self.advance(); }
            self.need_sign(table.single("syntax.call.close").unwrap(), "after the range")?;
            (start, end)
        } else {
            let tier = table.strings("op.range").iter().filter_map(|r| table.precedence.get(r.as_str())).min().copied().unwrap_or(0);
            let beginning = self.pos;
            let start = if self.on_any("ext.syntax.array.spread") { self.comma_value()? }
                else { let item = self.expr(tier + 1)?; self.comma_tail(item)? };
            if !(self.look().shape == Shape::Sign && table.spells("op.range", &self.look().lexeme)) {
                // No range mark: what was read is something to walk through.
                if !table.flag("ext.stmt.for.collection") {
                    return Err("A for loop needs a range: start..end".to_string());
                }
                let start = if asynchronous { prim_call(Prim::AsyncWalk, vec![start]) } else { start };
                let bounds = self.span_since(beginning);
                let start = self.located(bounds, start);
                let source = match table.single("ext.op.comprehension.for") {
                    Some(_) => prim_call(Prim::Iterated, vec![start]),
                    None => start,
                };
                self.claim(&var);
                self.address_to_write(&var);
                let walk = self.walk(source, None, var, None, place)?;
                return Ok(self.located(bounds, walk));
            }
            self.advance();
            let end = self.expr(tier + 1)?;
            (start, end)
        };
        self.claim(&var);
        self.address_to_write(&var);
        self.count_loop(&var, start, end, |r| r.body())
    }

    fn bind(&mut self) -> Res<Form> {
        if self.table.flag("stmt.let.type_first") {
            self.advance();
            let name = self.need_word("after the type")?;
            if let Some(open) = self.table.single("syntax.call.open") {
                if self.sign(open) {
                    return self.func(name, true);
                }
            }
            let value = if self.on_stmt_end() || self.exhausted() {
                constant(Value::Nil)
            } else {
                self.need_assign("in a declaration")?;
                self.expr(0)?
            };
            return Ok(self.write(&name, value));
        }
        self.advance();
        if self.key("stmt.let.mutable") {
            self.advance();
        }
        let name = self.need_word("after the binding keyword")?;
        if self.look().shape == Shape::Sign && self.table.spells("stmt.let.annotation", &self.look().lexeme) {
            self.advance();
            self.need_word("as a type name")?;
        }
        let value = if self.on_stmt_end() || self.exhausted() {
            constant(Value::Nil)
        } else {
            self.need_assign("in a binding")?;
            self.expr(0)?
        };
        Ok(self.write(&name, value))
    }

    fn on_assign(&self) -> bool {
        self.look().shape == Shape::Sign && self.table.spells("stmt.assign", &self.look().lexeme)
    }

    fn need_assign(&mut self, why: &str) -> Res<()> {
        if !self.table.has_any("stmt.assign") {
            return Err("This language has no assignment operator".to_string());
        }
        if self.on_assign() {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", self.table.single("stmt.assign").unwrap(), why, self.look().lexeme))
        }
    }


    /// The parameters of a function or a method, up to the closing bracket.
    fn parameters(&mut self, named: &str, short: bool) -> Res<(Vec<String>, Vec<(usize, usize)>, Vec<String>, Vec<Form>)> {
        let table = self.table;
        let close = if short { table.strings("ext.stmt.function.short")[1].clone() }
            else { table.single("syntax.call.close").unwrap().to_string() };
        let typed = table.flag("stmt.let.type_first");
        let mut params = Vec::new();
        let bind = table.flag("ext.syntax.call.bind_names");
        let mut manners: Vec<char> = Vec::new();
        let mut beyond = false;
        let mut slash = false;
        let mut closed = false;
        let mut optional = false;
        let wrong = || table.single("ext.stmt.function.parameters.amiss").unwrap_or("").to_string();
        // Which parameter, and where its own value stands: it is read
        // again inside the program, where its names mean what they should.
        let mut spares: Vec<(usize, usize)> = Vec::new();
        // A parameter with a class modifier before it names a property
        // of the thing as well, which the maker fills in.
        let mut also_property: Vec<String> = Vec::new();
        let mut repeated: Option<(String, usize)> = None;
        // Words said about how the parameters are written, said where
        // the routine is declared.
        let mut said: Vec<Form> = Vec::new();
        while !self.sign(&close) && !self.exhausted() {
            let mut manner = if beyond { 'n' } else { 'b' };
            if bind {
                if closed { return Err(if table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: parameters cannot follow var-keyword parameter".to_owned() } else { wrong() }); }
                let word = self.look().lexeme.clone();
                if table.spells("ext.stmt.function.positional_only", &word) {
                    if slash || beyond || params.is_empty() {
                        let complaint = if slash { "/ may appear only once" } else if beyond { "/ must be ahead of *" } else { "at least one parameter must precede /" };
                        return Err(if table.has_any("ext.builtin.exceptions.syntax") { format!("SyntaxError: {complaint}") } else { wrong() });
                    }
                    for before in &mut manners { *before = 'p'; }
                    slash = true;
                    self.advance();
                    if !self.sign(&close) {
                        if table.has_any("ext.builtin.exceptions.syntax") && self.sign("*") {
                            return Err("SyntaxError: expected comma between / and *".to_owned());
                        }
                        self.need_sign(table.single("syntax.call.separator").unwrap_or(""), "after the positional mark")?;
                    }
                    continue;
                }
                if table.spells("ext.stmt.function.carries.pairs", &word) {
                    closed = true;
                    manner = 'k';
                    self.advance();
                } else if table.spells("ext.stmt.function.carries", &word) || table.spells("ext.stmt.function.keyword_only", &word) {
                    if beyond { return Err(if table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: * may appear only once".to_owned() } else { wrong() }); }
                    if self.table.has_any("ext.builtin.exceptions.syntax") && (self.glance(1).shape == Shape::Sign && self.glance(1).lexeme == close) {
                        return Err("SyntaxError: named arguments must follow bare *".to_string());
                    }
                    beyond = true;
                    self.advance();
                    let separator = table.single("syntax.call.separator").unwrap_or("");
                    if self.sign(separator) {
                        self.advance();
                        if self.sign(&close) || table.spells("ext.stmt.function.carries.pairs", &self.look().lexeme) { return Err(wrong()); }
                        continue;
                    }
                    manner = 'v';
                }
            }
            let mut takes_nothing = false;
            let mut kinded = false;
            self.skip_reference();
            let mut for_the_thing = false;
            while self.look().shape == Shape::Bare && self.key("ext.stmt.class.modifier") {
                self.advance();
                for_the_thing = true;
            }
            if typed {
                let t = self.need_word("as a parameter type")?;
                if !table.spells("stmt.let", &t) {
                    return Err(format!("'{}' is not a type word", t));
                }
                if self.look().shape == Shape::Bare {
                    params.push(self.advance().lexeme);
                }
            } else {
                // A type may stand before the name, and may be marked as
                // taking nothing as well: `?int $x`.
                takes_nothing = self.skip_nothing_mark();
                let mut kind = None;
                // The mark that hands a cell over may stand between the
                // kind and the name: `foo &$a`.
                let next = self.glance(1);
                let between = next.shape == Shape::Sign
                    && table.single("ext.op.reference").map_or(false, |m| next.lexeme == m)
                    && self.glance(2).shape == Shape::Bare;
                if self.look().shape == Shape::Bare && (self.glance(1).shape == Shape::Bare || between) {
                    kind = Some(Rc::from(self.advance().lexeme.as_str()));
                    kinded = true;
                    if between {
                        self.skip_reference();
                    }
                }
                self.formal_kinds.push(kind);
                if table.has_any("ext.builtin.exceptions.syntax") && self.sign("(") {
                    return Err("SyntaxError: Function parameters cannot be parenthesized".to_owned());
                }
                params.push(self.need_word("as a parameter name")?);
                if table.has_any("ext.builtin.exceptions.syntax") && params.last().is_some_and(|word| word == "__debug__") { return Err("SyntaxError: cannot assign to __debug__".to_owned()); }
                if !short && self.on_any("ext.stmt.annotation") {
                    self.advance();
                    self.annotation_sites.push((params.last().unwrap().to_owned(), self.pos));
                    self.put_by_annotation(&["stmt.assign", "syntax.call.close", "syntax.call.separator"])?;
                } else if self.look().shape == Shape::Sign && table.spells("stmt.let.annotation", &self.look().lexeme) {
                    self.advance();
                    self.need_word("as a type name")?;
                }
            }
            if bind {
                let last = params.last().ok_or_else(wrong)?;
                if params.iter().filter(|p| *p == last).count() != 1 {
                    let twice = table.strings("ext.stmt.function.parameters.duplicate");
                    if twice.is_empty() { return Err(wrong()); }
                    let message = format!("{}{}{}", twice[0], last, twice.get(1).map_or("", String::as_str));
                    if table.has_any("ext.builtin.exceptions.syntax") {
                        let mut cursor = self.pos;
                        while cursor > 0 { cursor -= 1; if self.tokens[cursor].lexeme == *last { break; } }
                        repeated.get_or_insert((message, cursor));
                    } else {
                        return Err(message);
                    }
                }
                match (self.on_assign(), manner) {
                    (true, 'v' | 'k') => {
                        let kind = if manner == 'v' { "var-positional" } else { "var-keyword" };
                        return Err(if table.has_any("ext.builtin.exceptions.syntax") { format!("SyntaxError: {kind} parameter cannot have default value") } else { wrong() });
                    }
                    (true, 'b') => optional = true,
                    (false, 'b') if optional => {
                        if table.has_any("ext.builtin.exceptions.syntax") {
                            // The complaint names the parameter that
                            // takes nothing, standing over it as the
                            // reference stands.
                            let newest = params.last().cloned().unwrap_or_default();
                            let mut cursor = self.pos;
                            while cursor > 0 { cursor -= 1; if self.tokens[cursor].lexeme == newest { self.pos = cursor; break; } }
                            return Err("SyntaxError: parameter without a default follows parameter with a default".to_string());
                        }
                        return Err(wrong());
                    }
                    _ => {}
                }
                manners.push(manner);
            }
            if for_the_thing {
                also_property.push(params.last().expect("the parameter just read").clone());
            }
            // A parameter may carry a value of its own for calls that
            // leave it out.
            if self.on_assign() {
                let row = (self.look().row as u32).saturating_sub(self.before);
                self.advance();
                if table.has_any("ext.builtin.exceptions.syntax") && (self.sign(&close) || self.on_any("syntax.call.separator")) {
                    return Err("SyntaxError: expected default value expression".to_owned());
                }
                spares.push((params.len() - 1, self.pos));
                // Read once here only to step over it.
                let spare = self.expr(0)?;
                // A parameter written with a kind and nothing to fall
                // back on takes nothing as well as that kind, which the
                // reference asks to be written out rather than left to
                // be understood.
                let nothing = matches!(spare, Form::Const(Value::Nil));
                // A parameter written with a class's name takes a thing
                // of that class, and nothing else may stand for what it
                // falls back on. Only a value written out is told apart
                // here, and the reading stops over it as a fault of the
                // run.
                if let (Some(kind), Form::Const(worth)) = (self.formal_kinds.last().and_then(Clone::clone), &spare) {
                    if !nothing && names_a_class(table, &kind) {
                        self.stopped_fatally = true;
                        return Err(format!(
                            "Cannot use {} as default value for parameter {} of type {}",
                            kind_word(table, worth),
                            params.last().map_or("", String::as_str),
                            kind
                        ));
                    }
                }
                if nothing && !takes_nothing && kinded && self.tells_place {
                    let whose = match &self.within {
                        Some((class, _)) => format!("{}::{}", class, named),
                        None => named.to_string(),
                    };
                    let told = format!(
                        "{}(): Implicitly marking parameter {} as nullable is deprecated, the explicit nullable type must be used instead",
                        whose,
                        params.last().map_or("", String::as_str)
                    );
                    said.push(Form::Remark("deprecated", Rc::from(told.as_str()), row));
                }
            }
            if let Some(sep) = table.single("syntax.call.separator") {
                if self.sign(sep) {
                    self.advance();
                } else if bind && !self.sign(&close) { return Err(wrong()); }
            }
            if self.look().shape == Shape::Sign && table.separates(&self.look().lexeme) {
                self.advance();
            }
        }
        self.need_sign(&close, "after parameters")?;
        if let Some((message, at)) = repeated { self.pos = at; return Err(message); }
        self.taking = if bind { Some(manners) } else { None };
        Ok((params, spares, also_property, said))
    }

    /// Pass over the kind written beside a name. Its brackets shelter
    /// their contents from the marks ending the surrounding declaration.
    /// A class body's annotation as a routine of no parameters that
    /// answers the annotation's value, made where the body runs so that
    /// it is bound to the body's frame.
    fn annotation_routine(&mut self) -> Res<Form> {
        let ident = self.table.strings("ext.stmt.function.anonymous").first().map(String::as_str).unwrap_or(ANONYMOUS);
        // A plain expression: an equals sign after it gives the member its
        // value and is no assignment within the annotation.
        self.routine(ident, Holds::Every, Traps::Naught, Vec::new(), 0, |b| b.expr_at(0, false))
    }

    /// The key a blueprint carries a member's annotation under: a name
    /// beginning with two underscores and not ending so is mangled with
    /// the class's name, the reference's way with private names.
    fn private_key(&self, member: &str) -> String {
        let class = self.within.as_ref().map(|(name, _)| name.trim_start_matches('_')).unwrap_or("");
        if member.starts_with("__") && !member.ends_with("__") && !class.is_empty() { format!("_{class}{member}") } else { member.to_owned() }
    }

    fn put_by_annotation(&mut self, boundaries: &[&str]) -> Res<()> {
        let table = self.table;
        let mut nesting = Vec::new();
        let mut count = 0;
        loop {
            let shape = self.look().shape;
            if shape == Shape::Finish {
                break;
            }
            if nesting.is_empty() {
                let boundary = boundaries.iter().any(|label| self.on_any(label));
                if boundary || self.on_stmt_end() || matches!(shape, Shape::Open | Shape::Close) {
                    break;
                }
            }
            if self.forbids_await && self.key("ext.op.await") { return Err("SyntaxError: 'await' outside function".into()); }
            if table.has_any("ext.builtin.exceptions.syntax") && shape == Shape::Bare && self.key("ext.stmt.yield")
                && (self.in_class_body() || !self.layers.iter().skip(1).any(|s| s.holds == Holds::Every)) {
                let wording = if table.spells("ext.stmt.yield.from", &self.glance(1).lexeme) { "yield from" } else { "yield" };
                return Err(format!("SyntaxError: '{wording}' outside function"));
            }
            if shape == Shape::Sign {
                let spelling = self.look().lexeme.as_str();
                let mut opener = None;
                let mut is_close = false;
                for (left, right) in [("syntax.group.open", "syntax.group.close"),
                    ("syntax.array.open", "syntax.array.close"), ("syntax.map.open", "syntax.map.close"),
                    ("syntax.call.open", "syntax.call.close"), ("op.index.open", "op.index.close")] {
                    if table.spells(left, spelling) {
                        opener = table.single(right).map(str::to_string);
                    }
                    is_close |= table.spells(right, spelling);
                }
                match opener {
                    Some(end) => nesting.push(end),
                    None if is_close => {
                        if nesting.pop().as_deref() != Some(spelling) {
                            return Err(table.single("ext.stmt.annotation.amiss").unwrap_or("Expected an expression").into());
                        }
                    }
                    None => {}
                }
            }
            count += 1;
            self.advance();
        }
        if count > 0 && nesting.is_empty() {
            Ok(())
        } else {
            Err(table.single("ext.stmt.annotation.amiss").unwrap_or("Expected an expression").to_string())
        }
    }

    /// Step over the sign saying a type takes nothing as well: `?int`.
    fn skip_nothing_mark(&mut self) -> bool {
        let marks = self.table.strings("ext.op.ternary");
        let marked = marks.first().map_or(false, |q| self.sign(q));
        if marked && self.glance(1).shape == Shape::Bare {
            self.advance();
            return true;
        }
        false
    }

    /// Step over the sign saying a name shares a cell: where it stands
    /// was read before anything was built.
    /// The mark saying a routine gives back a cell and not a copy, if it
    /// stands here. Whether it did is given back.
    fn skip_reference(&mut self) -> bool {
        if self.table.single("ext.op.reference").map_or(false, |m| self.sign(m)) {
            self.advance();
            return true;
        }
        false
    }

    /// What a program does first when some of its parameters carry a
    /// value of their own: write each one the call left out.
    fn spare_values(&mut self, spares: Vec<(usize, usize)>, formals: &[String]) -> Res<Vec<Form>> {
        let held = self.pos;
        let mut first = Vec::new();
        for (at, from) in spares {
            self.pos = from;
            let value = self.expr(0)?;
            let slot = self.address_to_write(&formals[at]);
            let written = Form::Write(slot.clone(), Box::new(value));
            let test = Form::Missing(slot);
            first.push(self.choose(test, written, constant(Value::Nil)));
        }
        self.pos = held;
        Ok(first)
    }

    fn func(&mut self, name: String, bound: bool) -> Res<Form> {
        if bound && self.table.has_any("ext.builtin.exceptions.syntax") && name == "__debug__" { return Err("SyntaxError: cannot assign to __debug__".to_owned()); }
        let title = match self.pos.checked_sub(1).and_then(|at| self.original_words.get(at)) {
            Some(word) if word.shape == Shape::Bare && self.tokens[self.pos - 1].lexeme == name => word.lexeme.clone(),
            _ => name.clone(),
        };
        let deferred = self.pos.checked_sub(3).and_then(|at| self.tokens.get(at))
            .map_or(false, |word| self.table.spells("ext.stmt.async", &word.lexeme));
        self.type_names()?;
        // The type names read here stand aside while the parameters
        // and their defaults are read, so a routine written within one
        // of those is not handed them instead.
        let typed_names = std::mem::take(&mut self.pending_types);
        if bound && matches!(self.table.prims.get(&name), Some(Prim::Octets(_))) {
            self.arg_names.entry(name.to_owned()).or_insert_with(Vec::new);
        }
        let table = self.table;
        self.declared_at = (self.look().row as u32).saturating_sub(self.before);
        let open = table.single("syntax.call.open").ok_or_else(|| "This language has no call syntax".to_string())?;
        self.need_sign(open, "after function name")?;
        let typed = table.flag("stmt.let.type_first");
        let (params, spares, _, said) = self.parameters(&name, false)?;
        let least = params.len() - spares.len();
        let formals = params.clone();
        let keep_defaults = table.flag("ext.syntax.call.bind_names");
        let mut default_values = Vec::new();
        if keep_defaults {
            let resume = self.pos;
            for (_, start) in &spares {
                self.pos = *start;
                default_values.push(self.expr(0)?);
            }
            self.pos = resume;
        }
        let returns_here = |b: &Self| {
            b.look().shape == Shape::Sign
                && (table.spells("stmt.function.returns", &b.look().lexeme) || table.spells("ext.stmt.function.returns", &b.look().lexeme))
        };
        if returns_here(self) {
            self.advance();
            match table.has_any("ext.stmt.annotation") {
                true => { self.annotation_sites.push(("return".to_owned(), self.pos)); self.put_by_annotation(&["block.intro"])?; },
                false => {
                    self.skip_nothing_mark();
                    self.need_word("as a return type")?;
                }
            }
        }
        // A routine written where a value stands may take names from
        // around it away with it: the names around it are gone by the
        // time anybody calls it.
        let carried = self.carried_names()?;
        let taken = carried.clone();
        let declared = self.look().shape == Shape::Sign && table.separates(&self.look().lexeme);
        let statics_before = self.statics.len();
        self.pending_types = typed_names;
        let program = self.routine(&title, Holds::Every, Traps::Yields, params, least, |r| {
            r.layers.last_mut().unwrap().permits_async = deferred;
            r.generator_seen = deferred;
            // The names taken away sit in the slots after the
            // parameters, filled from what was taken when it is called.
            for (named, _) in &taken {
                let slot = r.address_to_write(named);
                r.carrying.push(slot.at);
            }
            let mut items = if keep_defaults {
                for (place, _) in spares { r.carrying.push(place); }
                Vec::new()
            } else { r.spare_values(spares, &formals)? };
            if declared {
                loop {
                    r.skip_line_ends();
                    if !typed && r.key("stmt.let") {
                        items.push(r.bind()?);
                    } else {
                        break;
                    }
                }
            }
            items.push(r.body()?);
            // A Python function without an explicit return yields None,
            // regardless of the value its last statement evaluated to.
            if table.has_any("ext.builtin.compile.modes") {
                items.push(constant(Value::Nil));
            }
            Ok(sequence(items))
        })?;
        let mut items: Vec<Form> = said;
        items.extend(self.statics.drain(statics_before..));
        // A routine written where a value stands is bound to no name and
        // stands for itself; one written out is bound to its name.
        // What is taken away is read where the routine stands, and the
        // routine carries it off.
        let program = match carried.is_empty() && default_values.is_empty() {
            true => program,
            false => {
                let mut given = vec![program];
                for (named, by_cell) in &carried {
                    given.push(match by_cell {
                        true => {
                            let shared = self.address_to_write(named);
                            Form::Share(shared)
                        }
                        false => self.read(named),
                    });
                }
                given.extend(default_values);
                prim_call(Prim::Carry, given)
            }
        };
        items.push(match bound {
            false => program,
            // A language may bind every routine among the outermost
            // bindings, wherever it is written, so one written inside
            // another is there for the whole run once that one has run.
            true => match self.table.flag("ext.stmt.function.outermost") {
                true => {
                    let slot = self.global_address(&name);
                    Form::Write(slot, Box::new(program))
                }
                false => self.write(&name, program),
            },
        });
        Ok(sequence(items))
    }

    /// A lambda gives its single expression back. Defaults keep their
    /// values; free bindings are found through the enclosing frames.
    fn lambda_form(&mut self) -> Res<Form> {
        let table = self.table;
        let keyword_at = self.pos.saturating_sub(1);
        let colon = table.single("block.intro").ok_or("Lambda needs a body mark")?;
        let comma = table.single("syntax.call.separator").ok_or("Lambda needs a parameter separator")?;
        let mut names = Vec::new();
        let mut ways = Vec::new();
        let mut positional_mark = false;
        let mut spares = Vec::new();
        let mut before = Vec::new();
        let mut gather = None;
        let mut named_only = false;
        let mut pairs = false;
        let mut default_seen = false;
        let mut repeated: Option<(String, usize)> = None;
        let bad = || table.single("ext.stmt.function.parameters.amiss").unwrap_or_default().to_string();
        loop {
            if self.sign(colon) { self.advance(); break; }
            if pairs { return Err(if table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: parameters cannot follow var-keyword parameter".to_owned() } else { bad() }); }
            if self.exhausted() { return Err("Expected lambda body".to_string()); }
            if table.spells("op.div", &self.look().lexeme) {
                if table.flag("ext.stmt.function.closes_over") {
                    if positional_mark || named_only || names.is_empty() {
                        let reason = if positional_mark { "/ may appear only once" } else if named_only { "/ must be ahead of *" } else { "at least one parameter must precede /" };
                        return Err(if table.has_any("ext.builtin.exceptions.syntax") { format!("SyntaxError: {reason}") } else { bad() });
                    }
                    positional_mark = true;
                    for way in &mut ways { *way = 'p'; }
                }
                self.advance();
                if table.has_any("ext.builtin.exceptions.syntax") && self.sign("*") {
                    return Err("SyntaxError: expected comma between / and *".to_owned());
                }
            } else {
                let many = table.spells("op.mul", &self.look().lexeme);
                let mapping = table.spells("op.pow", &self.look().lexeme);
                let mut way = if named_only { 'n' } else { 'b' };
                if many || mapping {
                    if many && named_only { return Err(if table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: * may appear only once".to_owned() } else { bad() }); }
                    self.advance();
                    named_only = true;
                    pairs = mapping;
                    way = if mapping { 'k' } else { 'v' };
                    if many && self.sign(comma) {
                        self.advance();
                        if self.sign(colon) { return Err(if table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: named parameters must follow bare *".to_owned() } else { bad() }); }
                        continue;
                    }
                    if many && table.has_any("ext.builtin.exceptions.syntax") && self.sign(colon) {
                        return Err("SyntaxError: named parameters must follow bare *".to_owned());
                    }
                    gather = Some(names.len());
                }
                if table.has_any("ext.builtin.exceptions.syntax") && self.sign("(") {
                    return Err("SyntaxError: Lambda expression parameters cannot be parenthesized".to_owned());
                }
                let parameter = self.need_word("as a lambda parameter")?;
                if table.has_any("ext.builtin.exceptions.syntax") && parameter == "__debug__" {
                    return Err("SyntaxError: cannot assign to __debug__".to_owned());
                }
                if names.contains(&parameter) {
                    let twice = table.strings("ext.stmt.function.parameters.duplicate");
                    if !twice.is_empty() && table.has_any("ext.builtin.exceptions.syntax") {
                        repeated.get_or_insert((format!("{}{}{}", twice[0], parameter, twice.get(1).map_or("", String::as_str)), self.pos - 1));
                    }
                    if !table.has_any("ext.builtin.exceptions.syntax") { return Err("Duplicate lambda parameter".to_string()); }
                }
                names.push(parameter);
                ways.push(way);
                if self.on_assign() {
                    if many || mapping {
                        let kind = if many { "var-positional" } else { "var-keyword" };
                        return Err(if table.has_any("ext.builtin.exceptions.syntax") { format!("SyntaxError: {kind} parameter cannot have default value") } else { bad() });
                    }
                    if way == 'b' { default_seen = true; }
                    self.advance();
                    if table.has_any("ext.builtin.exceptions.syntax") && (self.sign(colon) || self.sign(comma)) {
                        return Err("SyntaxError: expected default value expression".to_owned());
                    }
                    let worth = self.expr(0)?;
                    let hidden = self.gensym("spare");
                    before.push(Form::Write(hidden.clone(), Box::new(worth)));
                    spares.push((names.len() - 1, hidden));
                } else if way == 'b' && default_seen {
                    if table.has_any("ext.builtin.exceptions.syntax") {
                        let newest = names.last().cloned().unwrap_or_default();
                        let mut cursor = self.pos;
                        while cursor > 0 { cursor -= 1; if self.tokens[cursor].lexeme == newest { self.pos = cursor; break; } }
                        return Err("SyntaxError: parameter without a default follows parameter with a default".to_string());
                    }
                    return Err(bad());
                }
            }
            if !self.sign(colon) { self.need_sign(comma, "between lambda parameters")?; }
        }
        if let Some((message, at)) = repeated { self.pos = at; return Err(message); }
        let required = gather.unwrap_or(names.len()).saturating_sub(spares.len());
        let parameters = names.clone();
        let bind_arguments = table.flag("ext.syntax.call.bind_names");
        if bind_arguments { self.taking = Some(ways); }
        let enclosing: Vec<String> = self.layers.iter().skip(1).filter(|l| l.holds == Holds::Every).flat_map(|l| l.idents.clone()).collect();
        let mut unavailable = self.outside_lambda.clone();
        unavailable.extend(enclosing);
        let prior = std::mem::replace(&mut self.outside_lambda, unavailable);
        self.declared_at = (self.tokens[keyword_at].row as u32).saturating_sub(self.before);
        let lambda_ident = table.strings("ext.stmt.function.anonymous").first().map(String::as_str).unwrap_or(ANONYMOUS);
        let mut function = self.routine(lambda_ident, Holds::Every, Traps::Yields, names, required, |b| {
            let mut steps = Vec::new();
            for (index, source) in &spares {
                if bind_arguments { b.carrying.push(*index); continue; }
                let cell = b.address_to_write(&source.ident);
                b.carrying.push(cell.at);
                let target = b.address_to_write(&parameters[*index]);
                let absent = Form::Missing(target.clone());
                let fill = Form::Write(target, Box::new(Form::Read(cell)));
                steps.push(b.choose(absent, fill, constant(Value::Nil)));
            }
            let body = b.expr_at(0, !table.has_any("ext.builtin.exceptions.syntax"))?;
            if table.has_any("ext.builtin.exceptions.syntax") && b.on_assign() {
                let tail = &b.tokens[b.pos - 1];
                b.range_end = Some((tail.column + tail.lexeme.chars().count(), tail.row));
                b.pos = keyword_at;
                return Err(String::from("SyntaxError: cannot assign to lambda"));
            }
            // A lambda gives its expression back exactly as `return
            // expr` would: falling off a generator's body with the
            // value merely computed, and not handed back through the
            // same escape a `return` statement takes, would lose it
            // (a lambda generator's exhaustion then answers `None`
            // where the expression's own value belongs).
            steps.push(prim_call(Prim::Yield, vec![body]));
            Ok(sequence(steps))
        })?;
        self.outside_lambda = prior;
        if let Form::Const(Value::Routine(routine)) = &mut function {
            Rc::get_mut(routine).expect("new lambda").gather_from = if bind_arguments { None } else { gather };
        }
        if !spares.is_empty() {
            let mut carrying = vec![function];
            carrying.extend(spares.into_iter().map(|(_, slot)| Form::Read(slot)));
            function = prim_call(Prim::Carry, carrying);
        }
        before.push(function);
        Ok(sequence(before))
    }

    /// A routine written short: its parameters, the mark, and the one
    /// expression it answers with. Every name standing around it goes
    /// with it as it stands, since a routine written this short has
    /// nowhere to say which of them it wants.
    fn short_func(&mut self) -> Res<Form> {
        let table = self.table;
        self.declared_at = (self.look().row as u32).saturating_sub(self.before);
        if table.flag("ext.syntax.call.bind_names") {
            let (params, spares, _, _) = self.parameters(ANONYMOUS, true)?;
            let manners = self.taking.take();
            let body_at = self.pos;
            let mut held = Vec::new();
            for (_, from) in &spares {
                self.pos = *from;
                held.push(self.expr(0)?);
            }
            self.pos = body_at;
            self.taking = manners;
            let least = params.len() - spares.len();
            let program = self.routine(ANONYMOUS, Holds::Every, Traps::Yields, params, least, |r| {
                for (place, _) in &spares { r.carrying.push(*place); }
                r.expr(0)
            })?;
            if held.is_empty() { return Ok(program); }
            held.insert(0, program);
            return Ok(prim_call(Prim::Carry, held));
        }
        let open = table.single("syntax.call.open").ok_or_else(|| "This language has no call syntax".to_string())?;
        self.need_sign(open, "after the word for a short routine")?;
        let (params, spares, _, said) = self.parameters(ANONYMOUS, false)?;
        let least = params.len() - spares.len();
        let formals = params.clone();
        let returns_here = self.look().shape == Shape::Sign
            && (table.spells("stmt.function.returns", &self.look().lexeme) || table.spells("ext.stmt.function.returns", &self.look().lexeme));
        if returns_here {
            self.advance();
            match table.has_any("ext.stmt.annotation") {
                true => { self.annotation_sites.push(("return".to_owned(), self.pos)); self.put_by_annotation(&["block.intro"])?; },
                false => {
                    self.skip_nothing_mark();
                    self.need_word("as a return type")?;
                }
            }
        }
        let mark = table.strings("ext.stmt.function.short").get(1).cloned().ok_or_else(|| "A short routine needs a mark before its body".to_string())?;
        self.need_sign(&mark, "before the body of a short routine")?;
        // The names standing around it, save the ones it names itself
        // and the ones the kernel keeps for its own working.
        // A routine written this short has nowhere to say which names it
        // wants, so its body is read once to find out: it is read
        // through, the names it writes are noted, what came of it is
        // thrown away, and it is read again for the routine itself.
        let began = self.pos;
        let statics_here = self.statics.len();
        let wanted = {
            let taking = params.clone();
            let over = spares.clone();
            let named_here = formals.clone();
            self.routine(ANONYMOUS, Holds::Every, Traps::Yields, taking, least, |r| {
                let mut items = r.spare_values(over, &named_here)?;
                items.push(r.expr(0)?);
                Ok(sequence(items))
            })?;
            let ended = self.pos;
            let sigil = table.letter("identifier.variable_prefix");
            let mut names = std::collections::BTreeSet::new();
            for tok in &self.tokens[began..ended] {
                let a_binding = tok.shape == Shape::Bare && sigil.map_or(false, |m| tok.lexeme.starts_with(m));
                if a_binding && !formals.contains(&tok.lexeme) {
                    names.insert(tok.lexeme.clone());
                }
            }
            names
        };
        self.statics.truncate(statics_here);
        self.pos = began;
        let carried: Vec<(String, bool)> = wanted.into_iter().map(|named| (named, false)).collect();
        let taken = carried.clone();
        let program = self.routine(ANONYMOUS, Holds::Every, Traps::Yields, params, least, |r| {
            for (named, _) in &taken {
                let slot = r.address_to_write(named);
                r.carrying.push(slot.at);
            }
            let mut items = r.spare_values(spares, &formals)?;
            // The one expression stands where it is written, so whatever
            // is said while it runs names that line and not the line the
            // call was made from.
            let row = (r.look().row as u32).saturating_sub(r.before);
            let body = r.expr(0)?;
            items.push(Form::OnLine(row, Box::new(body)));
            Ok(sequence(items))
        })?;
        let program = match carried.is_empty() {
            true => program,
            false => {
                let mut given = vec![program];
                for (named, _) in &carried {
                    // A name never written to goes with it as nothing at
                    // all: the routine was never told to want it, so the
                    // taking says nothing of it, and reading it inside is
                    // what says it was never written.
                    let slot = self.address_to_read(named);
                    given.push(Form::Glance(slot));
                }
                prim_call(Prim::Carry, given)
            }
        };
        let mut items: Vec<Form> = said;
        items.push(program);
        Ok(sequence(items))
    }

    /// `use ($a, &$b)` after a routine written where a value stands: the
    /// names it takes away with it, and whether each is taken as it
    /// stands or as the cell it shares with the name it came from.
    fn carried_names(&mut self) -> Res<Vec<(String, bool)>> {
        let table = self.table;
        let words = table.strings("ext.stmt.function.carries");
        if words.is_empty() || !(self.look().shape == Shape::Bare && words.iter().any(|w| *w == self.look().lexeme)) {
            return Ok(Vec::new());
        }
        self.advance();
        let open = table.single("syntax.call.open").ok_or_else(|| "This language has no call syntax".to_string())?;
        let close = table.single("syntax.call.close").ok_or_else(|| "This language has no call syntax".to_string())?;
        let between = table.single("syntax.call.separator").map(str::to_string);
        self.need_sign(open, "after the word for what a routine takes away")?;
        let mut names = Vec::new();
        while !self.sign(close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            let by_cell = self.skip_reference();
            let named = self.need_word("as a name to take away")?;
            names.push((named, by_cell));
            if let Some(sep) = &between {
                if self.sign(sep) {
                    self.advance();
                }
            }
        }
        self.need_sign(close, "after what a routine takes away")?;
        Ok(names)
    }

    /// A declaration with its kind put by. A name alone has no work;
    /// a subscript without a value still works out its two parts.
    fn divided_at(&self, start: usize, limit: usize, label: &str) -> Vec<usize> {
        let mut depth: Vec<String> = Vec::new();
        let mut cuts = Vec::new();
        let t = self.table;
        for i in start..limit {
            let word = &self.tokens[i];
            if depth.is_empty() {
                if word.shape == Shape::Bare && t.spells("ext.op.lambda", &word.lexeme) { break; }
                if matches!(word.shape, Shape::Finish | Shape::Close | Shape::LineEnd) { break; }
                if word.shape == Shape::Sign && t.separates(&word.lexeme) { break; }
                if matches!(word.shape, Shape::Bare | Shape::Sign) && t.spells(label, &word.lexeme) { cuts.push(i); }
            }
            if word.shape == Shape::Sign {
                if depth.last().map(String::as_str) == Some(word.lexeme.as_str()) {
                    depth.pop();
                    continue;
                }
                for family in ["syntax.group", "syntax.array", "syntax.map"] {
                    if t.single(&format!("{}.open", family)) == Some(word.lexeme.as_str()) {
                        if let Some(close) = t.single(&format!("{}.close", family)) { depth.push(close.to_string()); }
                        break;
                    }
                }
            }
        }
        cuts
    }
    fn comma_part(&mut self, writes: bool) -> Res<(Form, bool)> {
        let star = self.table.single("ext.op.tuple").is_some() && self.on_any("ext.stmt.unpack.rest");
        if star { self.advance(); }
        let value = self.expr_at(0, writes)?;
        let gathered = if star {
            let partition = prim_call(Prim::Partition(1, Some(0)), vec![value]);
            prim_call(Prim::Apart, vec![partition, constant(Value::Small(0))])
        } else { value };
        Ok((gathered, star))
    }

    fn has_added_target(&self, span: std::ops::Range<usize>) -> bool {
        let mut inside = Vec::new();
        for index in span.clone() {
            let token = &self.tokens[index];
            if token.lexeme == "[" || token.lexeme == "(" {
                let carries = inside.iter().any(|scope| *scope);
                let indexing = index > span.start && self.tokens[index - 1].shape == Shape::Bare;
                inside.push(carries || indexing);
            } else if token.lexeme == "]" || token.lexeme == ")" {
                inside.pop();
            } else if token.lexeme == "+" && !inside.iter().any(|scope| *scope) {
                return true;
            }
        }
        false
    }

    fn loop_target_error(&self, span: std::ops::Range<usize>, error: String) -> String {
        if !self.table.has_any("ext.builtin.exceptions.syntax") || !error.starts_with("SyntaxError:") { return error; }
        if self.has_added_target(span.clone()) {
            return String::from("SyntaxError: cannot assign to expression");
        }
        if self.tokens[span].windows(2).any(|two| two[0].shape == Shape::Bare && two[1].lexeme == "(") {
            return String::from("SyntaxError: cannot assign to function call");
        }
        error
    }

    /// A target is read afresh at its turn, after the whole right hand
    /// side has been kept. Nested targets each check their own extent.
    fn distribute(&mut self, span: std::ops::Range<usize>, source: &str) -> Res<Form> {
        let bad = self.table.single("ext.stmt.unpack.amiss").unwrap_or("Invalid assignment target").to_string();
        let (mut lo, mut hi) = (span.start, span.end);
        if lo >= hi { return Err(bad); }
        if self.table.has_any("ext.builtin.exceptions.syntax") && hi == lo + 1 {
            let entry = &self.tokens[lo];
            let reserved = entry.lexeme == "__debug__" || ["literal.true", "literal.false", "literal.null"]
                .iter().any(|label| self.table.spells(label, &entry.lexeme));
            let description = if reserved { Some(entry.lexeme.as_str()) }
                else if matches!(entry.shape, Shape::Quote | Shape::ByteQuote | Shape::Numeral) { Some("literal") }
                else { None };
            if let Some(description) = description {
                return Err(format!("SyntaxError: cannot assign to {description}"));
            }
        }
        let mut array = false;
        while lo < hi {
            let token = &self.tokens[lo];
            let family = if token.shape != Shape::Sign { break; }
                else if self.table.spells("syntax.array.open", &token.lexeme) { "syntax.array" }
                else if self.table.spells("syntax.group.open", &token.lexeme) { "syntax.group" }
                else { break; };
            let ending = self.table.single(&format!("{}.close", family)).unwrap();
            let mut level = 1;
            let mut ends = lo + 1;
            while ends < hi {
                let next = &self.tokens[ends];
                if next.shape == Shape::Sign {
                    if next.lexeme == token.lexeme { level += 1; }
                    if next.lexeme == ending { level -= 1; }
                }
                if level == 0 { break; }
                ends += 1;
            }
            if ends + 1 != hi { break; }
            if self.table.has_any("ext.builtin.exceptions.syntax") && !self.divided_at(lo + 1, hi - 1, "ext.op.comprehension.for").is_empty() {
                return Err(String::from(if family == "syntax.array" {
                    "SyntaxError: cannot assign to list comprehension here. Maybe you meant '==' instead of '='?"
                } else { "SyntaxError: cannot assign to generator expression" }));
            }
            array |= family == "syntax.array";
            lo += 1;
            hi -= 1;
            if lo == hi { array = true; }
            if array { break; }
            if !self.divided_at(lo, hi, "ext.op.tuple").is_empty() { break; }
        }
        let cuts = self.divided_at(lo, hi, "ext.op.tuple");
        if cuts.is_empty() && !array {
            self.pos = lo;
            self.place_depth += 1;
            let reading = if hi == lo + 1 && self.look().shape == Shape::Bare {
                let word = self.advance().lexeme;
                if ["literal.true", "literal.false", "literal.null"].iter().any(|label| self.table.spells(label, &word)) {
                    Err("Invalid assignment target before '='".to_string())
                } else { Ok(self.read(&word)) }
            } else { self.monadic_expr() };
            self.place_depth -= 1;
            let place = reading?;
            if self.pos != hi { return Err(bad); }
            if self.layers.last().unwrap().gathering_kind.is_some() && self.table.has_any("ext.builtin.slice") {
                if let Form::Apply(Callee::Prim(Prim::At, _), mut operands) = place {
                    let container = self.gather_name("target_object");
                    let evaluate = self.write(&container, operands.remove(0));
                    operands.insert(0, self.read(&container));
                    operands.push(self.read(source));
                    return Ok(sequence(vec![evaluate, prim_call(Prim::Replace, operands)]));
                }
            }
            let sign = self.look().clone();
            let old = self.waiting.replace(source.to_string());
            let written = self.write_into(place, false, None, sign);
            self.waiting = old;
            return written;
        }
        if self.table.single("ext.stmt.unpack").is_none() { return Err(bad); }
        let mut pieces = Vec::new();
        let mut previous = lo;
        for boundary in cuts.into_iter().chain(Some(hi)) {
            if previous < boundary { pieces.push(previous..boundary); }
            else if boundary < hi { return Err(bad); }
            previous = boundary + 1;
        }
        let mut starred = None;
        for (number, piece) in pieces.iter_mut().enumerate() {
            let lead = &self.tokens[piece.start];
            if lead.shape == Shape::Sign && self.table.spells("ext.stmt.unpack.rest", &lead.lexeme) {
                if starred.is_some() { return Err(bad); }
                starred = Some(number);
                piece.start += 1;
            }
        }
        let held = self.gensym("partition").ident.to_string();
        let input = self.read(source);
        let checked = prim_call(Prim::Partition(pieces.len(), starred), vec![input]);
        let mut writes = vec![self.write(&held, checked)];
        for (index, part) in pieces.into_iter().enumerate() {
            let bag = self.read(&held);
            let value = prim_call(Prim::Apart, vec![bag, constant(Value::Small(index as i64))]);
            let item = self.gensym("part").ident.to_string();
            writes.push(self.write(&item, value));
            writes.push(self.distribute(part, &item)?);
        }
        Ok(sequence(writes))
    }
    fn chained_places(&mut self) -> Res<Option<Form>> {
        if self.table.single("ext.op.tuple").is_none() && !self.table.flag("ext.stmt.assign.chain") { return Ok(None); }
        let mut left = self.pos;
        let signs = self.divided_at(left, self.tokens.len(), "stmt.assign");
        if signs.is_empty() || (signs.len() > 1 && !self.table.flag("ext.stmt.assign.chain")) { return Ok(None); }
        if !self.divided_at(left, signs[0], "ext.stmt.annotation").is_empty() { return Ok(None); }
        let enclosed = self.on_any("syntax.group.open") || self.on_any("syntax.array.open");
        if signs.len() == 1 && !enclosed && self.divided_at(left, signs[0], "ext.op.tuple").is_empty() { return Ok(None); }
        if self.table.has_any("ext.builtin.exceptions.syntax") && self.has_added_target(left..signs[0]) {
            return Err(String::from("SyntaxError: cannot assign to expression"));
        }
        self.pos = signs[signs.len() - 1] + 1;
        let answer = self.comma_value()?;
        let resume = self.pos;
        let name = self.gensym("given").ident.to_string();
        let mut forms = vec![self.write(&name, answer)];
        for sign in signs {
            if self.table.spells("ext.stmt.yield", &self.tokens[left].lexeme) {
                return Err(String::from("SyntaxError: assignment to yield expression not possible"));
            }
            forms.push(self.distribute(left..sign, &name)?);
            left = sign + 1;
        }
        self.pos = resume;
        Ok(Some(sequence(forms)))
    }

    /// Where a second writing sign stands at the statement's own
    /// depth ahead: an annotated assignment takes the one sign only, a
    /// second, as `x: int = y = 1` has, being invalid syntax.
    fn second_sign_ahead(&self) -> Option<usize> {
        let mut depth = 0i32;
        let mut at = self.pos + 1;
        while let Some(token) = self.tokens.get(at) {
            if matches!(token.shape, Shape::LineEnd | Shape::Close | Shape::Finish | Shape::Open) { break; }
            if token.shape == Shape::Sign {
                if self.table.separates(&token.lexeme) || self.table.spells("block.intro", &token.lexeme) { break; }
                match token.lexeme.as_str() {
                    "(" | "[" | "{" => depth += 1,
                    ")" | "]" | "}" => depth -= 1,
                    _ if depth == 0 && (self.table.spells("stmt.assign", &token.lexeme) || self.table.compound.contains_key(&token.lexeme)) => return Some(at),
                    _ => {}
                }
            }
            at += 1;
        }
        None
    }

    fn with_annotation(&mut self, place: Form, began: usize) -> Res<Form> {
        let table = self.table;
        if !matches!(&place, Form::Read(_)
            | Form::Apply(Callee::Prim(Prim::At | Prim::Of, _), _)) {
            return Err(table.single("ext.stmt.annotation.amiss").unwrap_or("Expected an assignment target").to_string());
        }
        // A target in parentheses names nothing: no binding is made for
        // it, nothing is annotated, and it says nothing for a `global`
        // or `nonlocal` naming it later, as the reference has it.
        let parenthesized = self.tokens.get(began).map_or(false, |t| t.shape == Shape::Sign && t.lexeme == "(");
        if let Form::Read(slot) = &place {
            let named = slot.ident.to_string();
            self.unmet_last_read(&named);
            if self.table.has_any("ext.builtin.exceptions.syntax") && named == "__debug__" {
                return Err(String::from("SyntaxError: cannot assign to __debug__"));
            }
            if !parenthesized {
                self.mark_met(&named, MET_ANNOTATED);
                let declared = self.declarations.iter().find(|(_, word, _)| word == &named).map(|(_, _, outer)| *outer);
                let class_global = declared.is_none()
                    && self.class_globals.last().filter(|(level, _)| *level == self.layers.len()).map_or(false, |(_, names)| names.iter().any(|n| n == &named));
                if self.table.has_any("ext.builtin.exceptions.syntax") && (declared.is_some() || class_global) {
                    let token = self.tokens[began].clone();
                    self.range_end = Some((token.column + token.lexeme.chars().count(), token.row));
                    self.pos = began;
                    let which = if declared == Some(false) { "nonlocal" } else { "global" };
                    return Err(format!("SyntaxError: annotated name '{named}' can't be {which}"));
                }
            }
        }
        if self.layers.len() == 1 && !parenthesized {
            if let Form::Read(slot) = &place {
                if self.tokens[self.pos - 1].lexeme == slot.ident.as_ref() {
                    self.module_sites.push((slot.ident.to_string(), self.pos + 1));
                }
            }
        }
        self.advance();
        self.put_by_annotation(&["stmt.assign", "ext.stmt.annotation", "syntax.call.separator"])?;
        if self.on_assign() {
            if self.table.has_any("ext.builtin.exceptions.syntax") {
                if let Some(at) = self.second_sign_ahead() {
                    self.pos = at;
                    return Err("SyntaxError: invalid syntax".to_owned());
                }
            }
            return self.written(place, false);
        }
        Ok(match place {
            Form::Read(slot) => {
                if table.flag("ext.stmt.function.closes_over") && !parenthesized { self.address_to_write(&slot.ident); }
                constant(Value::Nil)
            }
            Form::Apply(Callee::Prim(Prim::At, _), parts) => sequence(parts),
            Form::Apply(Callee::Prim(Prim::Of, _), mut parts) => parts.remove(0),
            _ => unreachable!("the annotation target was read above"),
        })
    }

    /// Seek the kind's sign before reading a statement's place. Marks
    /// sheltered by brackets are part of the place, not its declaration.
    fn declaration_mark(&self) -> Option<usize> {
        if !self.table.has_any("ext.stmt.annotation") { return None; }
        let mut closings = Vec::new();
        let mut at = self.pos;
        while let Some(token) = self.tokens.get(at) {
            if closings.is_empty() {
                match token.shape {
                    Shape::Sign if self.table.spells("ext.stmt.annotation", &token.lexeme) => return Some(at),
                    Shape::LineEnd | Shape::Open | Shape::Close | Shape::Finish => return None,
                    _ => {}
                }
                if ["stmt.assign", "stmt.terminator"].iter().any(|label| self.table.spells(label, &token.lexeme)) {
                    return None;
                }
            }
            if token.shape == Shape::Sign {
                if closings.last().copied() == Some(token.lexeme.as_str()) { closings.pop(); }
                else {
                    for (open, close) in [("syntax.group.open", "syntax.group.close"),
                        ("syntax.array.open", "syntax.array.close"), ("syntax.map.open", "syntax.map.close"),
                        ("syntax.call.open", "syntax.call.close"), ("op.index.open", "op.index.close")] {
                        if self.table.spells(open, &token.lexeme) {
                            if let Some(end) = self.table.single(close) { closings.push(end); }
                            break;
                        }
                    }
                }
            }
            at += 1;
        }
        None
    }

    fn write_or_expr(&mut self) -> Res<Form> {
        let word = self.look().lexeme.to_owned();
        let unchanged = self.layers.len() == 1 && !self.in_class_body()
            && self.table.flag("ext.syntax.names.shadow_builtins")
            && (self.table.prims.contains_key(&word) || self.table.spells("ext.stmt.class.parent", &word))
            && self.glance(1).shape == Shape::Sign && self.table.spells("stmt.assign", &self.glance(1).lexeme)
            && self.glance(2).shape == Shape::Bare && self.glance(2).lexeme == word
            && matches!(self.glance(3).shape, Shape::LineEnd | Shape::Finish)
            && (self.native_exports.contains(&word) || !self.named_in_program.contains(&word));
        let built = self.binding_or_value()?;
        if unchanged { self.native_exports.insert(word); }
        Ok(built)
    }

    fn interactive_statement(&self, value: Form) -> Form {
        if self.interactive && self.layers.len() == 1 && !self.in_class_body() {
            prim_call(Prim::Display, vec![value])
        } else {
            value
        }
    }

    fn binding_or_value(&mut self) -> Res<Form> {
        if self.table.has_any("ext.builtin.exceptions.syntax") {
            if let Some(&equal) = self.divided_at(self.pos, self.tokens.len(), "stmt.assign").first() {
                let word = self.look();
                let target = if word.shape == Shape::Woven {
                    Some(if self.table.spells("ext.lexical.string.prefix.template", &word.lexeme) { "t-string expression" } else { "f-string expression" })
                } else if word.lexeme == "{" && self.pair_close(self.pos, equal) == Some(equal - 1) {
                        Some(if self.divided_at(self.pos + 1, equal - 1, "syntax.map.pair").is_empty() { "set display" } else { "dict literal" })
                    } else { None };
                if word.shape == Shape::Bare && self.glance(1).lexeme == "if" {
                    return Err(String::from("SyntaxError: cannot assign to conditional expression"));
                }
                if let Some(description) = target {
                    return Err(format!("SyntaxError: cannot assign to {description} here. Maybe you meant '==' instead of '='?"));
                }
            }
            if self.look().lexeme == "None" && self.table.compound.contains_key(&self.glance(1).lexeme) {
                return Err(String::from("SyntaxError: 'None' is an illegal expression for augmented assignment"));
            }
            let list = self.on_any("syntax.array.open");
            let call = self.look().shape == Shape::Bare && self.glance(1).lexeme == "(";
            if list || call {
                let began = self.pos + usize::from(call);
                if let Some(end) = self.pair_close(began, self.tokens.len()) {
                    if self.tokens.get(end + 1).is_some_and(|next| self.table.compound.contains_key(&next.lexeme)) {
                        let name = if list { "list" } else { "function call" };
                        return Err(format!("SyntaxError: '{name}' is an illegal expression for augmented assignment"));
                    }
                }
            }
            if self.look().lexeme == "__debug__" && self.table.compound.contains_key(&self.glance(1).lexeme) {
                return Err(String::from("SyntaxError: cannot assign to __debug__"));
            }
            if let Some(mark) = self.declaration_mark() {
                if mark > self.pos + 1 && self.tokens[mark - 1].lexeme == "__debug__" && self.tokens[mark - 2].lexeme == "." {
                    return Err(String::from("SyntaxError: cannot assign to __debug__"));
                }
                let (mut left, mut right) = (self.pos, mark);
                while left + 1 < right && self.tokens[left].lexeme == "(" && self.tokens[right - 1].lexeme == ")" { left += 1; right -= 1; }
                if left == right || !self.divided_at(left, right, "ext.op.tuple").is_empty() {
                    return Err(String::from("SyntaxError: only single target (not tuple) can be annotated"));
                }
                if self.tokens[left].lexeme == "[" && self.pair_close(left, right) == Some(right - 1) {
                    return Err(String::from("SyntaxError: only single target (not list) can be annotated"));
                }
                let starts = &self.tokens[left];
                let comprehension = self.tokens[self.pos].lexeme == "(" && self.tokens[left..mark].iter().any(|part| part.lexeme == "for");
                if comprehension || starts.lexeme == "-" || matches!(starts.shape, Shape::Numeral | Shape::Quote | Shape::ByteQuote) {
                    return Err(String::from("SyntaxError: illegal target for annotation"));
                }
            }
            let signs = self.divided_at(self.pos, self.tokens.len(), "stmt.assign");
            if let Some(&assignment) = signs.first() {
                let (mut head, mut tail) = (self.pos, assignment);
                while head + 1 < tail && self.tokens[head].lexeme == "(" && self.tokens[tail - 1].lexeme == ")" {
                    head += 1; tail -= 1;
                }
                let single = head + 1 == tail;
                let token = &self.tokens[head];
                let initial = &token.lexeme;
                let bad: Option<String> = if self.table.spells("ext.stmt.function.short", initial) { Some("cannot assign to lambda".to_owned()) }
                    else if self.table.spells("ext.stmt.yield", initial) { Some(if single { "assignment to yield expression not possible".to_owned() } else { "cannot assign to yield expression here. Maybe you meant '==' instead of '='?".to_owned() }) }
                    else if single && matches!(token.shape, Shape::Numeral | Shape::Quote | Shape::ByteQuote) { Some("cannot assign to literal here. Maybe you meant '==' instead of '='?".to_owned()) }
                    else if single && self.table.spells("ext.literal.ellipsis", initial) { Some("cannot assign to ellipsis here. Maybe you meant '==' instead of '='?".to_owned()) }
                    else if single && token.shape == Shape::Bare && (self.table.spells("literal.null", initial) || self.table.spells("literal.true", initial) || self.table.spells("literal.false", initial)) { Some(format!("cannot assign to {initial}")) }
                    else if single && token.shape == Shape::Bare && initial == "__debug__" && token.row as u32 > self.before { Some("cannot assign to __debug__".to_owned()) }
                    else { None };
                if let Some(bad) = bad {
                    let ending = &self.tokens[tail - 1];
                    self.range_end = Some((ending.column + ending.lexeme.chars().count(), ending.row));
                    self.pos = head;
                    return Err(format!("SyntaxError: {bad}"));
                }
            }
        }
        if let Some(write) = self.chained_places()? { return Ok(write); }
        if self.divided_at(self.pos, self.tokens.len(), "stmt.assign").is_empty()
            && !self.divided_at(self.pos, self.tokens.len(), "ext.op.tuple").is_empty() {
            let tuple = self.comma_expression(false)?;
            return if self.on_writing() {
                Err(String::from("SyntaxError: 'tuple' is an illegal expression for augmented assignment"))
            } else { Ok(self.interactive_statement(tuple)) };
        }
        let began = self.pos;
        let boundary = began.checked_sub(1).map_or(true, |at| {
            let prior = &self.tokens[at];
            match prior.shape {
                Shape::LineEnd | Shape::Open | Shape::Close => true,
                Shape::Sign => ["stmt.terminator", "block.intro"].iter()
                    .any(|label| self.table.spells(label, &prior.lexeme)),
                _ => false,
            }
        });
        let writing = !self.divided_at(self.pos, self.tokens.len(), "stmt.assign").is_empty();
        self.place_depth += usize::from(writing);
        let enclosing_mark = self.kind_mark.take();
        if boundary { self.kind_mark = self.declaration_mark(); }
        let read = self.expr_at(0, false);
        self.kind_mark = enclosing_mark;
        self.place_depth -= usize::from(writing);
        let mut expr = read?;
        if self.on_writing() && self.table.has_any("ext.stmt.class.special") {
            if self.tokens.get(began + 1).map_or(false, |token| self.table.spells("op.pipe", &token.lexeme)) {
                let assignment = self.pos;
                self.pos = began;
                expr = self.deletion_place()?;
                self.pos = assignment;
            }
        }
        if self.on_writing() && !self.on_assign() && self.table.has_any("ext.builtin.exceptions.syntax") {
            let first = self.tokens[began..self.pos].iter().find(|token| token.lexeme != "(");
            if first.is_some_and(|token| self.table.spells("ext.stmt.yield", &token.lexeme)) {
                return Err(String::from("SyntaxError: 'yield expression' is an illegal expression for augmented assignment"));
            }
        }
        let follows = |reader: &Self| reader.on_assign() && reader.glance(1).shape == Shape::Bare
            && reader.glance(2).shape == Shape::Sign && reader.table.spells("stmt.assign", &reader.glance(2).lexeme);
        if self.table.flag("ext.stmt.assign.chain") && follows(self) {
            if let Form::Read(first) = &expr {
                let mut destinations = vec![first.ident.to_string()];
                self.unmet_last_read(&destinations[0].clone());
                while follows(self) { self.advance(); destinations.push(self.advance().lexeme); }
                self.advance();
                let answer = self.comma_value()?;
                let saved = self.gensym("chain_value");
                let mut steps = vec![Form::Write(saved.clone(), Box::new(answer))];
                for destination in destinations { steps.push(self.write(&destination, Form::Read(saved.clone()))); }
                return Ok(sequence(steps));
            }
        }
        if self.on_any("ext.op.tuple") {
            let _target = self.comma_tail(expr)?;
            if self.on_assign() && self.table.flag("ext.stmt.yield.suspends") {
                self.advance();
                let value = self.comma_value()?;
                let held = self.gensym("unpacked");
                let name = held.ident.to_string();
                let mut parts = vec![Form::Write(held, Box::new(value))];
                let after = self.pos;
                self.pos = began;
                parts.extend(self.loop_targets(&name)?);
                self.pos = after;
                return Ok(sequence(parts));
            }
            if self.on_assign() { self.advance(); let _value = self.comma_value()?; }
            return Ok(self.scope_unrun("ext.system.scope.unready"));
        }
        if boundary && self.on_any("ext.stmt.annotation") {
            return self.with_annotation(expr, began);
        }
        if !self.on_writing() {
            let value = self.comma_tail(expr)?;
            return Ok(self.interactive_statement(value));
        }
        let tail = &self.tokens[began..self.pos];
        let attribute = tail.len() >= 2
            && tail[tail.len() - 1].shape == Shape::Bare
            && tail[tail.len() - 2].shape == Shape::Sign
            && self.table.spells("op.pipe", &tail[tail.len() - 2].lexeme);
        let temporary_index = matches!(&expr,
            Form::Apply(Callee::Prim(Prim::At, _), args)
                if matches!(args.first(), Some(Form::Apply(Callee::Code(_), _))));
        // Attributes and call results cannot yet retain writes in these scopes.
        if (attribute && !self.table.has_any("ext.op.member") || temporary_index)
            && self.table.has_any("ext.system.scope.unready") {
            self.advance();
            let _right = self.comma_value()?;
            return Ok(self.scope_unrun("ext.system.scope.unready"));
        }
        self.written(expr, false)
    }

    /// Whether a sign that writes stands here: the assignment sign, or
    /// an operator run together with it.
    fn on_writing(&self) -> bool {
        let compound = self.look().shape == Shape::Sign && self.table.compound.contains_key(&self.look().lexeme);
        self.on_assign() || compound
    }

    /// A write of what follows the sign into the target already read.
    /// Where the write is itself an expression, what was written is kept
    /// in a cell of its own and given back once the writing is done.
    fn written(&mut self, expr: Form, gives_back: bool) -> Res<Form> {
        let compound = if self.look().shape == Shape::Sign { self.table.compound.get(&self.look().lexeme).copied() } else { None };
        let compound = if self.table.flag("ext.syntax.set") {
            compound.map(|p| match p { Prim::BitsBoth => Prim::SetAssign(1), Prim::Minus => Prim::SetAssign(2), Prim::BitsEither => Prim::SetAssign(0), Prim::BitsOne => Prim::SetAssign(3), p => p })
        } else { compound };
        let assign = self.advance();
        let refused_tuple = matches!(&expr, Form::Apply(Callee::Prim(Prim::Raise, _), parts)
            if matches!(parts.as_slice(), [Form::Const(Value::Text(words))]
                if self.table.single("ext.system.scope.unready") == Some(words.as_ref())));
        if refused_tuple {
            let _ = self.comma_value()?;
            return Ok(expr);
        }
        self.write_into(expr, gives_back, compound, assign)
    }

    /// The write itself, the sign that called for it already read.
    /// The value a place holds just before a step, kept where the step
    /// asked for it, so that `p++` may stand for what was there.
    fn kept_before(&mut self, found: Form) -> Form {
        match self.stood.clone() {
            Some(cell) => {
                let stored = self.write(&cell, found);
                sequence(vec![stored, self.read(&cell)])
            }
            None => found,
        }
    }

    /// The same for the value the place holds once the step is done.
    fn kept_after(&mut self, made: Form) -> Form {
        // Only a compound form asks for the newer spelling. A plain
        // working has no call here and retains its former words.
        let made = if self.stepping.is_none() && self.table.flag("ext.builtin.print.real_point") {
            prim_call(Prim::Pointed, vec![made])
        } else {
            made
        };
        match self.stands.clone() {
            Some(cell) => {
                let stored = self.write(&cell, made);
                sequence(vec![stored, self.read(&cell)])
            }
            None => made,
        }
    }

    /// The value a compound write lands on, kept aside where the write
    /// itself stands for a value, so what follows reads that and not
    /// whatever the writing itself answered with.
    fn kept_landing(&mut self, gives_back: bool, made: Form) -> (Form, Option<String>) {
        if !gives_back {
            return (made, None);
        }
        self.gensyms += 1;
        let cell = format!("#landed{}", self.gensyms);
        let stored = self.write(&cell, made);
        let back = self.read(&cell);
        (sequence(vec![stored, back]), Some(cell))
    }

    fn binding_chain(&mut self, first: &str, answers: bool) -> Res<Form> {
        let mut targets = vec![self.address_to_write(first)];
        loop {
            if self.look().shape != Shape::Bare
                || !self.table.spells("stmt.assign", &self.glance(1).lexeme) { break; }
            let target_name = self.advance().lexeme;
            if ["literal.true", "literal.false", "literal.null"].iter().any(|label| self.table.spells(label, &target_name)) {
                return Err("Invalid assignment target before '='".to_string());
            }
            targets.push(self.address_to_write(&target_name));
            self.advance();
        }
        let value = self.comma_value()?;
        let saved = self.gensym("chained");
        let mut steps = vec![Form::Write(saved.clone(), Box::new(value))];
        for place in targets {
            steps.push(Form::Write(place, Box::new(Form::Read(saved.clone()))));
        }
        if answers { steps.push(Form::Read(saved)); }
        Ok(sequence(steps))
    }

    fn write_into(&mut self, expr: Form, gives_back: bool, compound: Option<Prim>, assign: Token) -> Res<Form> {
        let compound = compound.map(|op| {
            if self.table.has_any("ext.builtin.bytes") && matches!(op, Prim::Plus | Prim::Times) { Prim::OctetAssign(op == Prim::Times) }
            else { op }
        });
        // Where the special list reaches the in-place methods, the
        // working is numbered so the place written to is asked first.
        let compound = compound.map(|op| self.table.landing_place(op).map_or(op, Prim::Landing));
        // A target kept quiet is a write kept quiet: the muting comes
        // off the reading and goes round the writing instead.
        if let Form::Silenced(inner) = expr {
            let written = self.write_into(*inner, gives_back, compound, assign)?;
            return Ok(Form::Silenced(Box::new(written)));
        }
        if let Form::Muted(inner) = expr {
            let written = self.write_into(*inner, gives_back, compound, assign)?;
            return Ok(Form::Muted(Box::new(written)));
        }
        // A bare name read only to be written over was no use of the
        // name: the write says it was written, as the reference counts
        // it against a later declaration.
        if let Form::Read(slot) = &expr {
            let named = slot.ident.to_string();
            self.unmet_last_read(&named);
        }
        // The name a method knows its own thing by is bound by the call
        // and by nothing else: a language naming one turns a write to it
        // down outright, and calls that a fault of the run.
        if let (Some(this), Form::Read(slot)) = (self.table.single("ext.stmt.class.this"), &expr) {
            if slot.ident.as_ref() == this {
                self.stopped_fatally = true;
                return Err(format!("Cannot re-assign {}", this));
            }
        }
        if self.table.flag("ext.stmt.assign.names.chained") && compound.is_none()
            && self.waiting.is_none() && self.stepping.is_none()
            && self.look().shape == Shape::Bare
            && self.table.spells("stmt.assign", &self.glance(1).lexeme) {
            if let Form::Read(slot) = &expr {
                return self.binding_chain(&slot.ident, gives_back);
            }
        }
        let plain = compound.is_none();
        // `b = &a`: b is tied to a's cell rather than given a copy.
        let tied_to_a_cell = self.table.single("ext.op.reference").map_or(false, |m| self.sign(m)) && plain;
        let mut shared_value: Option<Form> = None;
        if tied_to_a_cell && matches!(expr, Form::Apply(Callee::Prim(Prim::At, _), _)) {
            self.advance();
            shared_value = Some(self.a_shared_cell(&self.table.strings("ext.op.reference.unshared.written").to_vec(), false, None)?);
        }
        if self.table.single("ext.op.reference").map_or(false, |m| self.sign(m)) && plain {
            if let Form::Read(slot) = &expr {
                let held = slot.ident.to_string();
                self.advance();
                let shared = self.a_shared_cell(&self.table.strings("ext.op.reference.unshared.written").to_vec(), false, None)?;
                let tied = self.address_to_write(&held);
                let tie = Form::Tie(tied, Box::new(shared));
                return Ok(match gives_back {
                    true => sequence(vec![tie, self.read(&held)]),
                    false => tie,
                });
            }
            // `$GLOBALS['n'] = &e`: the binding the name spells is tied
            // to the cell, as a name written out would be.
            if let Form::Called(spells) = expr {
                self.advance();
                let shared = self.a_shared_cell(&self.table.strings("ext.op.reference.unshared.written").to_vec(), false, None)?;
                return Ok(Form::TieCalled(spells, Box::new(shared)));
            }
        }
        // Where the value is already worked out and waiting in a cell,
        // the write reads it from there rather than reading what comes
        // after the sign: a taking-apart has no sign before each place.
        // Where a cell was already taken, that cell is the value.
        let mut value = match (self.stepping, self.waiting.clone(), &shared_value) {
            (_, _, Some(_)) => constant(Value::Nil),
            // A step carries its own value: the one it steps by, with no
            // source of its own after the sign.
            (Some(by), _, None) => constant(Value::Small(by)),
            (None, Some(cell), None) => self.read(&cell),
            (None, None, None) => if self.table.has_any("ext.op.tuple") { self.comma_value()? } else { self.expr(0)? },
        };
        // The value comes before the bounds of a slice assignment.
        let keyed_write = matches!(expr, Form::Apply(Callee::Prim(Prim::At, _), _)) && self.table.has_any("ext.builtin.slice");
        let before_bounds = if plain && (slice_target(&expr) || keyed_write) {
            self.gensyms += 1;
            let saved = format!("#slice_value{}", self.gensyms);
            let first = self.write(&saved, value);
            value = self.read(&saved);
            Some((first, saved))
        } else {
            None
        };
        // Where a language writes into text, a place there holds one
        // letter and no more, so a write into a single named place is
        // worth the letter that went in and not the whole of what was
        // handed over. Only what is written into can say whether this
        // is text, so it is read once, quietly, after the value: a name
        // holding nothing has nothing to answer for here.
        if plain && shared_value.is_none() && self.table.flag("ext.op.index.text") {
            let named = match &expr {
                Form::Apply(Callee::Prim(Prim::At, _), args) if args.len() == 2 => match args.first() {
                    Some(Form::Read(slot)) => Some(slot.ident.to_string()),
                    _ => None,
                },
                _ => None,
            };
            if let Some(named) = named {
                let into = Form::Muted(Box::new(self.read(&named)));
                value = prim_call(Prim::Letter, vec![value, into]);
            }
        }
        let keep = (gives_back && plain).then(|| {
            self.gensyms += 1;
            format!("#written{}", self.gensyms)
        });
        if let Some(cell) = &keep {
            let stored = self.write(cell, value);
            let back = self.read(cell);
            value = sequence(vec![stored, back]);
        }
        let (chain, expr) = match chain_apart(expr) {
            Ok(found) => (Some(found), Form::Const(Value::Nil)),
            Err(back) => (None, back),
        };
        // A chain of looks standing on something other than a bare name
        // is rebuilt whole and written back into what it stood on.
        let (footing, expr) = match chain {
            Some(_) => (None, expr),
            None => match footing_apart(expr) {
                Ok(found) => (Some(found), Form::Const(Value::Nil)),
                Err(back) => (None, back),
            },
        };
        let made = match expr {
            // x op= e is x = x op e.
            Form::Read(slot) => match compound {
                Some(op) => {
                    // x op= e is x = x op e, and yet the name is only
                    // ever written by it, as the reference counts it.
                    let current = self.read(&slot.ident);
                    self.unmet_last_read(&slot.ident);
                    let current = self.kept_before(current);
                    let combined = self.kept_after(prim_call(op, vec![current, value]));
                    let stored = self.write(&slot.ident, combined);
                    match gives_back {
                        true => sequence(vec![stored, self.read(&slot.ident)]),
                        false => stored,
                    }
                }
                None => self.write(&slot.ident, value),
            },
            // A chain standing on something other than a bare name: the
            // keys are worked out once and in order, the arrays along
            // the way rewritten from the innermost outwards, and the
            // whole written back into what it stood on.
            _ if footing.is_some() => {
                let (stands_on, keys, after) = footing.expect("what the chain stands on");
                let mut steps = Vec::new();
                let mut at_cells = Vec::new();
                for key in keys {
                    self.gensyms += 1;
                    let cell = format!("#key{}", self.gensyms);
                    let stored = self.write(&cell, key);
                    steps.push(stored);
                    at_cells.push(cell);
                }
                let deep = match after { true => at_cells.len(), false => at_cells.len() - 1 };
                let mut in_cells = Vec::new();
                self.gensyms += 1;
                let root = format!("#in{}", self.gensyms);
                self.gensyms += 1;
                let holding = format!("#value{}", self.gensyms);
                // The value is worked out while what the chain stands on
                // is still whole, since working it out may change that
                // very thing. A compound write cannot: it wants the
                // place read first, and so comes after the way in.
                let mut afterward = None;
                match compound {
                    None => steps.push(self.write(&holding, value)),
                    Some(_) => afterward = Some(value),
                }
                // What the chain stands on is taken as a cell, so that
                // rewriting the arrays within it lands where it lives
                // and nothing need be written back afterwards.
                let start = match stands_on {
                    place @ Form::Apply(Callee::Prim(Prim::Of | Prim::At | Prim::Within, _), _) => self.cell_of(place)?,
                    call @ Form::Apply(..) if self.table.flag("ext.syntax.call.bind_names") => {
                        let saved = self.gensym("target_result");
                        steps.push(Form::Write(saved.clone(), Box::new(call)));
                        self.cell_of(Form::Read(saved))?
                    }
                    other => self.cell_of(other)?,
                };
                // Tied, not written: a plain write of a cell writes what
                // it holds, and here the cell itself is wanted.
                let tied = self.address_to_write(&root);
                steps.push(Form::Tie(tied, Box::new(start)));
                in_cells.push(root);
                for i in 0..deep {
                    self.gensyms += 1;
                    let cell = format!("#in{}", self.gensyms);
                    let (so_far, key) = (self.read(&in_cells[i]), self.read(&at_cells[i]));
                    let further = prim_call(Prim::Inward, vec![so_far, key]);
                    steps.push(self.write(&cell, further));
                    in_cells.push(cell);
                }
                if let (Some(op), Some(value)) = (compound, afterward) {
                    let (lands_in, key) = (self.read(&in_cells[deep]), self.read(&at_cells[deep]));
                    let now = self.kept_before(prim_call(Prim::Toward, vec![lands_in, key]));
                    let combined = self.kept_after(prim_call(op, vec![now, value]));
                    steps.push(self.write(&holding, combined));
                }
                let lands_in = self.read(&in_cells[deep]);
                let put = self.read(&holding);
                steps.push(match after {
                    true => prim_call(Prim::Append, vec![lands_in, put]),
                    false => {
                        let key = self.read(&at_cells[deep]);
                        prim_call(Prim::Replace, vec![lands_in, key, put])
                    }
                });
                for i in (0..deep).rev() {
                    let (holds, key, done) = (self.read(&in_cells[i]), self.read(&at_cells[i]), self.read(&in_cells[i + 1]));
                    steps.push(prim_call(Prim::Restore, vec![holds, key, done]));
                }
                // The cells the rewriting stood on were scaffolding, and
                // are let go now the write has landed: a cell the program
                // itself does not share should not go on being held by a
                // name of the builder's own making.
                for cell in &in_cells {
                    let slot = self.address_to_write(cell);
                    steps.push(Form::Forget(slot));
                }
                if gives_back {
                    steps.push(self.read(&holding));
                }
                sequence(steps)
            }
            // `a[i][j] = v` and `a[i][] = v`: the keys are worked out
            // once and in order, the arrays along the way kept, and each
            // rewritten in the one above it. A place not there yet is
            // made on the way in, since that is what a write asks for.
            _ if chain.is_some() => {
                let (name, keys, after) = chain.expect("a chain on a name");
                let mut steps = Vec::new();
                let mut at_cells = Vec::new();
                for key in keys {
                    self.gensyms += 1;
                    let cell = format!("#key{}", self.gensyms);
                    let stored = self.write(&cell, key);
                    steps.push(stored);
                    at_cells.push(cell);
                }
                // In through the arrays, each one kept as far as the one
                // the write lands in.
                let deep = match after { true => at_cells.len(), false => at_cells.len() - 1 };
                let mut in_cells = Vec::new();
                self.gensyms += 1;
                let root = format!("#in{}", self.gensyms);
                self.gensyms += 1;
                let holding = format!("#value{}", self.gensyms);
                // The value comes first, while the name still holds what
                // it held: reading the array out to rewrite it would
                // leave the name empty while the value is worked out,
                // and working it out may change the array itself. A
                // compound write wants the place read first, and waits.
                let mut afterward = None;
                match compound {
                    None => steps.push(self.write(&holding, value)),
                    Some(_) => afterward = Some(value),
                }
                // Where a write makes the places it needs, the name it
                // starts from has nothing to say about being empty.
                let start = self.read_to_write(&name);
                let start = match self.table.flag("ext.op.index.makes") {
                    true => Form::Muted(Box::new(start)),
                    false => start,
                };
                steps.push(self.write(&root, start));
                in_cells.push(root);
                for i in 0..deep {
                    self.gensyms += 1;
                    let cell = format!("#in{}", self.gensyms);
                    let (so_far, key) = (self.read(&in_cells[i]), self.read(&at_cells[i]));
                    let further = prim_call(Prim::Inward, vec![so_far, key]);
                    steps.push(self.write(&cell, further));
                    in_cells.push(cell);
                }
                if let (Some(op), Some(value)) = (compound, afterward) {
                    let (lands_in, key) = (self.read(&in_cells[deep]), self.read(&at_cells[deep]));
                    let now = self.kept_before(prim_call(Prim::Toward, vec![lands_in, key]));
                    let combined = self.kept_after(prim_call(op, vec![now, value]));
                    steps.push(self.write(&holding, combined));
                }
                // Writing into a place changes the array the name holds
                // where it stands, so each array along the way is written
                // into the one above it, from the innermost outwards.
                let lands_in = self.read(&in_cells[deep]);
                let put = self.read(&holding);
                steps.push(match after {
                    true => prim_call(Prim::Append, vec![lands_in, put]),
                    false => {
                        let key = self.read(&at_cells[deep]);
                        prim_call(Prim::Replace, vec![lands_in, key, put])
                    }
                });
                for i in (0..deep).rev() {
                    let (holds, key, done) = (self.read(&in_cells[i]), self.read(&at_cells[i]), self.read(&in_cells[i + 1]));
                    steps.push(prim_call(Prim::Restore, vec![holds, key, done]));
                }
                let back = self.read(&in_cells[0]);
                let home = self.address_to_rewrite(&name);
                steps.push(Form::Write(home, Box::new(back)));
                if gives_back {
                    steps.push(self.read(&holding));
                }
                sequence(steps)
            }
            // `a[i] = &b`: the place holds the cell itself, so a write
            // through either name is a write the other sees.
            Form::Apply(Callee::Prim(Prim::At, _), ref args) if tied_to_a_cell && args.len() == 2 => {
                let Form::Apply(_, mut args) = expr else { unreachable!("a look") };
                let index = args.pop().expect("the place");
                match args.pop().expect("what holds it") {
                    Form::Read(slot) => {
                        let target = self.read_to_write(&slot.ident);
                        prim_call(Prim::Replace, vec![target, index, shared_value.expect("a cell")])
                    }
                    _ => return Err("Invalid assignment target".to_string()),
                }
            }
            // A read of the binding a value names becomes a write of it.
            Form::Called(spells) if compound.is_none() => Form::CallWrite(spells, Box::new(value)),
            // `p op= e` where the place is a member of something, one of
            // a class's own, or a single place in a named array: what
            // holds it and the name of the place are each worked out
            // once and kept, the place read from there, taken with the
            // value, and written back where it came from.
            Form::Apply(Callee::Prim(Prim::Of, _), mut args) if compound.is_some() && args.len() == 2 => {
                let op = compound.expect("the operation");
                let named = args.pop().expect("the property");
                let thing = args.pop().expect("what holds it");
                self.gensyms += 1;
                let holder = format!("#holder{}", self.gensyms);
                self.gensyms += 1;
                let called = format!("#called{}", self.gensyms);
                let hold = self.write(&holder, thing);
                let name = self.write(&called, named);
                let now = prim_call(Prim::Of, vec![self.read(&holder), self.read(&called)]);
                let now = self.kept_before(now);
                let made = self.kept_after(prim_call(op, vec![now, value]));
                let (made, landed) = self.kept_landing(gives_back, made);
                let put = prim_call(Prim::Onto, vec![self.read(&holder), self.read(&called), made]);
                let mut steps = vec![hold, name, put];
                if let Some(cell) = landed {
                    steps.push(self.read(&cell));
                }
                sequence(steps)
            }
            Form::Apply(Callee::Prim(Prim::Within, _), mut args) if compound.is_some() && args.len() == 2 => {
                let op = compound.expect("the operation");
                let named = args.pop().expect("the value's name");
                let class = args.pop().expect("the class");
                self.gensyms += 1;
                let holder = format!("#holder{}", self.gensyms);
                self.gensyms += 1;
                let called = format!("#called{}", self.gensyms);
                let hold = self.write(&holder, class);
                let name = self.write(&called, named);
                let now = prim_call(Prim::Within, vec![self.read(&holder), self.read(&called)]);
                let now = self.kept_before(now);
                let made = self.kept_after(prim_call(op, vec![now, value]));
                let (made, landed) = self.kept_landing(gives_back, made);
                let put = prim_call(Prim::Into, vec![self.read(&holder), self.read(&called), made]);
                let mut steps = vec![hold, name, put];
                if let Some(cell) = landed {
                    steps.push(self.read(&cell));
                }
                sequence(steps)
            }
            Form::Apply(Callee::Prim(Prim::At, _), mut args) if compound.is_some() && args.len() == 2 => {
                let op = compound.expect("the operation");
                let index = args.pop().expect("the place");
                let Form::Read(slot) = args.pop().expect("what holds it") else {
                    return Err("Invalid assignment target".to_string());
                };
                let held = slot.ident.to_string();
                self.gensyms += 1;
                let key = format!("#key{}", self.gensyms);
                let hold = self.write(&key, index);
                let now = prim_call(Prim::Toward, vec![self.read(&held), self.read(&key)]);
                let now = self.kept_before(now);
                let made = self.kept_after(prim_call(op, vec![now, value]));
                let (made, landed) = self.kept_landing(gives_back, made);
                let target = self.read_to_write(&held);
                let put = prim_call(Prim::Replace, vec![target, self.read(&key), made]);
                let mut steps = vec![hold, put];
                if let Some(cell) = landed {
                    steps.push(self.read(&cell));
                }
                sequence(steps)
            }
            // `$$x op= e`: the name is worked out once and held, the
            // binding it spells read under that name and what comes of
            // the working written back under it.
            Form::Called(spells) if compound.is_some() => {
                let op = compound.expect("the operation");
                self.gensyms += 1;
                let spelt = format!("#spelt{}", self.gensyms);
                let hold = self.write(&spelt, *spells);
                let now = Form::Called(Box::new(self.read(&spelt)));
                let now = self.kept_before(now);
                let made = self.kept_after(prim_call(op, vec![now, value]));
                let put = Form::CallWrite(Box::new(self.read(&spelt)), Box::new(made));
                sequence(vec![hold, put])
            }
            _ if compound.is_some() => return Err(format!("'{}' needs a plain variable on its left", assign.lexeme)),
            // A read of a member becomes a write of it.
            Form::Apply(Callee::Prim(Prim::Of, _), mut args) if args.len() == 2 => {
                let named = args.pop().unwrap();
                if self.table.has_any("ext.builtin.exceptions.syntax")
                    && matches!(&named, Form::Const(Value::Text(word)) if word.as_ref() == "__debug__") {
                    return Err(String::from("SyntaxError: cannot assign to __debug__"));
                }
                let thing = args.pop().unwrap();
                prim_call(Prim::Onto, vec![thing, named, value])
            }
            Form::Apply(Callee::Prim(Prim::Within, _), mut args) if args.len() == 2 => {
                let named = args.pop().unwrap();
                let class = args.pop().unwrap();
                prim_call(Prim::Into, vec![class, named, value])
            }
            // `t->a[i] = v` and `t->a[] = v`: the property's array is
            // rewritten and the property written back.
            Form::Apply(Callee::Prim(op @ (Prim::At | Prim::AtEnd), _), mut args) if matches!(args.first(), Some(Form::Apply(Callee::Prim(Prim::Of, _), _))) => {
                let index = if op == Prim::At { args.pop() } else { None };
                let Some(Form::Apply(_, mut inner)) = args.pop() else { unreachable!("a property read") };
                let Some(Form::Const(Value::Text(named))) = inner.pop() else {
                    return Err("Invalid assignment target".to_string());
                };
                let Some(Form::Read(slot)) = inner.pop() else {
                    return Err("Only a named thing's property may be written into".to_string());
                };
                let ident = slot.ident.to_string();
                let held = prim_call(Prim::Of, vec![self.read(&ident), constant(Value::text(&named))]);
                let written = match index {
                    Some(index) => prim_call(Prim::Placed, vec![held, index, value]),
                    None => prim_call(Prim::Added, vec![held, value]),
                };
                let target = self.read(&ident);
                prim_call(Prim::Onto, vec![target, constant(Value::text(&named)), written])
            }
            // a[] = v appends.
            Form::Apply(Callee::Prim(Prim::AtEnd, _), mut args) if args.len() == 1 => match args.pop().unwrap() {
                Form::Read(slot) => {
                    let target = self.read_to_write(&slot.ident);
                    prim_call(Prim::Append, vec![target, value])
                }
                // The binding the name spells is read quietly, since a
                // write makes what is not there yet, the value put after
                // its last place, and the whole written back under the
                // same name.
                Form::Called(spells) => {
                    self.gensyms += 1;
                    let named = format!("#named{}", self.gensyms);
                    let hold = self.write(&named, *spells);
                    let held = Form::Muted(Box::new(Form::Called(Box::new(self.read(&named)))));
                    let grown = prim_call(Prim::Added, vec![held, value]);
                    let put = Form::CallWrite(Box::new(self.read(&named)), Box::new(grown));
                    sequence(vec![hold, put])
                }
                _ => return Err("Invalid assignment target".to_string()),
            },
            Form::Apply(Callee::Prim(Prim::At, _), mut args) if args.len() == 2 => {
                let index = args.pop().unwrap();
                match args.pop().unwrap() {
                    Form::Read(slot) => {
                        let target = self.read_to_write(&slot.ident);
                        prim_call(Prim::Replace, vec![target, index, value])
                    }
                    _ => return Err("Invalid assignment target".to_string()),
                }
            }
            other if self.table.has_any("ext.builtin.exceptions.syntax") => {
                let kind = match other { Form::Apply(Callee::Code(_), _) => "function call", _ => "expression" };
                return Err(format!("SyntaxError: cannot assign to {kind} here. Maybe you meant '==' instead of '='?"));
            }
            _ => return Err(format!("Invalid assignment target before '{}'", assign.lexeme)),
        };
        // The value worked out ahead of the bounds is forgotten once
        // written, so that its last holder is never a hidden cell.
        let made = match before_bounds {
            Some((first, saved)) => sequence(vec![first, made, Form::Forget(self.address_to_write(&saved))]),
            None => made,
        };
        Ok(match keep {
            Some(cell) => sequence(vec![made, self.read(&cell)]),
            None => made,
        })
    }

    // ---------- expressions

    fn expr(&mut self, floor: u32) -> Res<Form> {
        self.expr_at(floor, true)
    }

    fn written_inside_display(&self, start: usize) -> bool {
        let mut scopes = Vec::new();
        let mut taking = Vec::new();
        for index in start..self.tokens.len() {
            let token = &self.tokens[index];
            if scopes.is_empty() && matches!(token.shape, Shape::LineEnd | Shape::Close | Shape::Finish) { break; }
            if token.lexeme == "lambda" { if let Some(formals) = taking.last_mut() { *formals = true; } }
            if token.shape != Shape::Sign { continue; }
            if token.lexeme == "(" {
                scopes.push(index > 0 && self.tokens[index - 1].lexeme != "assert" && (self.tokens[index - 1].shape == Shape::Bare || [")", "]"].contains(&self.tokens[index - 1].lexeme.as_str())));
                taking.push(false);
            } else if token.lexeme == "[" || token.lexeme == "{" {
                scopes.push(false);
                taking.push(false);
            } else if [")", "]", "}"].contains(&token.lexeme.as_str()) {
                scopes.pop();
                taking.pop();
                if scopes.is_empty() { break; }
            } else if token.lexeme == ":" {
                if let Some(formals) = taking.last_mut() { *formals = false; }
            } else if token.lexeme == "=" && index > start && self.tokens[index - 1].shape == Shape::Bare
                && scopes.last() == Some(&false) && taking.last() != Some(&true) {
                return true;
            }
        }
        false
    }

    /// An expression. Where a language counts a write as one, a target
    /// followed by a sign that writes is read as a write whose value is
    /// what was written — but not where the write is the whole
    /// statement, which is read as a statement.
    fn expr_at(&mut self, floor: u32, may_write: bool) -> Res<Form> {
        let table = self.table;
        let origin = self.pos;
        if floor == 0 && table.has_any("ext.builtin.exceptions.syntax")
            && matches!(self.look().lexeme.as_str(), "(" | "[" | "{") && self.written_inside_display(origin) {
            return Err(String::from("SyntaxError: invalid syntax. Maybe you meant '==' or ':=' instead of '='?"));
        }
        if floor == 0 && self.look().shape == Shape::Bare && table.spells("ext.op.assign.expression", &self.glance(1).lexeme) {
            let word = self.advance().lexeme;
            if table.has_any("ext.builtin.exceptions.syntax") {
                if table.spells("literal.true", &word) || table.spells("literal.false", &word) || table.spells("literal.null", &word) {
                    return Err(format!("SyntaxError: cannot use assignment expressions with {word}"));
                }
                if word == "__debug__" {
                    return Err("SyntaxError: cannot assign to __debug__".to_owned());
                }
            }
            let layer = self.layers.last_mut().unwrap();
            if layer.gathering_kind.is_some() { layer.expression_targets.push(word.clone()); }
            self.advance();
            let target = self.address_to_write(&word);
            let expression = self.expr(0)?;
            return Ok(sequence(vec![Form::Write(target.clone(), Box::new(expression)), Form::Read(target)]));
        }
        if floor == 0 && table.has_any("ext.builtin.exceptions.syntax")
            && matches!(self.look().lexeme.as_str(), "pass" | "break" | "continue")
            && self.glance(1).lexeme == "if" {
            return Err("SyntaxError: expected expression before 'if', but statement is given".to_owned());
        }
        let mut left = self.monadic_expr()?;
        if floor == 0 && table.spells("ext.op.assign.expression", &self.look().lexeme) {
            let Form::Read(target) = left else {
                return Err("Named expression needs a variable".to_string());
            };
            self.advance();
            let rhs = self.expr(0)?;
            let bind = self.write(&target.ident, rhs);
            let back = self.read(&target.ident);
            self.unmet_last_read(&target.ident);
            self.unmet_last_read(&target.ident);
            return Ok(sequence(vec![bind, back]));
        }
        if floor == 0 && may_write && table.flag("ext.op.assign.value") && self.on_writing() {
            if table.has_any("ext.builtin.exceptions.syntax") && origin + 1 == self.pos && matches!(self.tokens[origin].shape, Shape::Numeral | Shape::Quote | Shape::ByteQuote) {
                self.range_end = Some((self.look().column, self.look().row));
                self.pos = origin;
                return Err(String::from("SyntaxError: cannot assign to literal here. Maybe you meant '==' instead of '='?"));
            }
            return self.written(left, true);
        }
        loop {
            let t = self.look();
            if t.shape != Shape::Sign && t.shape != Shape::Bare {
                break;
            }
            let text = t.lexeme.clone();
            if table.has_any("ext.builtin.exceptions.syntax") && (text == "|" || text == "&") {
                let following = self.glance(1);
                if following.lexeme == text && (following.row, following.column) == (t.row, t.column + 1) {
                    self.range_end = Some((t.column + 2, t.row));
                    let keyword = if text == "|" { "or" } else { "and" };
                    return Err(format!("SyntaxError: invalid syntax. Maybe you meant '{keyword}' or '{text}' instead of '{text}{text}'?"));
                }
            }
            let conditional = table.strings("ext.op.if_else");
            if floor == 0 && conditional.first() == Some(&text) {
                self.advance();
                let test = self.expr(1)?;
                let end = conditional.get(1).ok_or("Conditional expression needs two words")?;
                if self.look().lexeme != *end {
                    return Err(if table.has_any("ext.builtin.exceptions.syntax") && !self.on_any("block.intro") {
                        String::from("SyntaxError: expected 'else' after 'if' expression")
                    } else { format!("Expected '{}' in conditional expression", end) });
                }
                self.advance();
                if table.has_any("ext.builtin.exceptions.syntax") && ["pass", "return", "raise", "del", "yield", "assert", "break", "continue", "import", "from"].contains(&self.look().lexeme.as_str()) {
                    return Err("SyntaxError: expected expression after 'else', but statement is given".to_owned());
                }
                let no = self.expr(0)?;
                left = self.choose(test, left, no);
                continue;
            }
            if table.flag("ext.op.compare.chained") {
                if let Some((operation, level, words)) = self.comparison_head() {
                    if floor > level { break; }
                    let saved = self.gensym("middle");
                    let keep = Form::Write(saved.clone(), Box::new(left));
                    let links = self.comparison_tail(saved.clone(), operation, level, words)?;
                    // The middle is forgotten once the links are judged,
                    // so that a value's last holder is never a cell no
                    // name reaches.
                    let judged = self.gensym("judged");
                    left = sequence(vec![keep, Form::Write(judged.clone(), Box::new(links)), Form::Forget(saved), Form::Read(judged)]);
                    continue;
                }
            }
            if table.spells("op.pipe", &text) {
                if table.precedence.get(&text).copied().unwrap_or(0) < floor {
                    break;
                }
                self.advance();
                let name = self.need_word("after the pipe")?;
                if let Some((label, _)) = crate::table::BUILTIN_LABELS.iter().find(|(label, prim)| *prim == Prim::ValueMethod && table.spells(label, &name)) {
                    let operation = label.strip_prefix("ext.builtin.method.").expect("a method label");
                    left = prim_call(Prim::BindValueMethod, vec![left, constant(Value::text(operation))]);
                    left = self.subscript(left)?;
                    continue;
                }
                let mut args = vec![left];
                let mut invoked = false;
                if let Some(open) = table.single("syntax.call.open") {
                    if self.sign(open) {
                        self.advance();
                        invoked = true;
                        args.extend(self.args("syntax.call.close", "syntax.call.separator")?);
                    }
                }
                if table.flag("ext.stmt.yield.suspends") && ["ext.stmt.yield.send", "ext.stmt.yield.close", "ext.stmt.yield.throw"].iter().any(|label| table.spells(label, &name)) {
                    args.insert(1, constant(Value::text(&name)));
                    left = prim_call(Prim::Ask, args);
                } else if matches!(table.prims.get(&name), Some(Prim::SetCall(1..=17))) {
                left = match table.prims.get(&name).copied() {
                    Some(Prim::SetCall(1..=17)) if !invoked => {
                        let words = table.single("ext.builtin.set.method.unavailable").unwrap_or_default();
                        args.push(prim_call(Prim::Raise, vec![constant(Value::text(words))]));
                        sequence(args)
                    }
                    Some(op @ Prim::SetCall(1..=17)) => Form::Apply(Callee::Prim(op, Rc::from(name.as_str())), args),
                    _ => self.named_call(&name, args)?,
                };
                } else {
                let mutation = matches!(table.prims.get(&name), Some(Prim::Append) | Some(Prim::Replace));
                left = if mutation && table.has_any("ext.system.scope.unready") && !matches!(args.first(), Some(Form::Read(_))) {
                    self.scope_unrun("ext.system.scope.unready")
                } else { self.named_call(&name, args)? };
                }
                if table.has_any("ext.system.scope.unready") { left = self.subscript(left)?; }
                continue;
            }
            if table.single("ext.op.otherwise").map_or(false, |m| self.sign(m)) {
                // `a ?? b`: b is a program, read only when a is nothing.
                // What stands on the left is read quietly: a name or a
                // place that is not there is the case the whole is
                // written for, and nothing to be told of.
                self.advance();
                let otherwise = self.limb(Traps::Naught, |r| r.expr(0))?;
                left = prim_call(Prim::Otherwise, vec![Form::Muted(Box::new(left)), otherwise]);
                continue;
            }
            if self.look().shape == Shape::Bare && table.spells("ext.op.instanceof", &text) {
                self.advance();
                let named = self.need_word("as the class to test against")?;
                // A class is usually written out where one is tested
                // against. Where a binding is written there instead, it
                // is read: what it holds spells the class, or is a thing
                // whose own class is the one meant.
                let a_binding = table.letter("identifier.variable_prefix").map_or(false, |mark| named.starts_with(mark));
                let against = match a_binding {
                    true => self.read(&named),
                    false => constant(Value::text(&named)),
                };
                left = prim_call(Prim::Akin, vec![left, against]);
                continue;
            }
            let head = self.comparison_head();
            let Some(mut op) = table.dyadic.get(&text).copied().or_else(|| {
                head.map(|(prim, level, _)| crate::table::Infix { prim, level, right_assoc: false })
            }) else {
                // test ? a : b, at the bottom of an expression.
                let signs = table.strings("ext.op.ternary");
                if floor == 0 && signs.first().map_or(false, |q| self.sign(q)) {
                    self.advance();
                    let then = self.limb(Traps::Naught, |r| r.expr(0))?;
                    self.need_sign(&signs[1], "between the arms of the conditional")?;
                    let otherwise = self.limb(Traps::Naught, |r| r.expr(0))?;
                    left = self.choose(left, then, otherwise);
                    continue;
                }
                break;
            };
            if op.level < floor {
                break;
            }
            let consumed = if let Some((prim, _, width)) = head { op.prim = prim; width } else { 1 };
            for _ in 0..consumed { self.advance(); }
            let floor_right = if op.right_assoc { op.level } else { op.level + 1 };
            left = match op.prim {
                // The right side is a program, run only when the left leaves it open.
                Prim::Both | Prim::Either => {
                    let lazy = self.limb(Traps::Naught, |r| r.expr(floor_right))?;
                    prim_call(op.prim, vec![left, lazy])
                }
                other => {
                    let right = self.expr(floor_right)?;
                    // A bare name is fetched when the operator falls,
                    // not when it stands, so a write on the far side is
                    // seen by it. Nothing else is fetched that late: a
                    // property, a class's own value, a place in an
                    // array or what a call gave back is set down where
                    // it stands, out of a later write's reach. So the
                    // far side is set down first and the name looked at
                    // after it, both operands then being addresses the
                    // operation reads for itself.
                    let late = match &left {
                        Form::Read(slot) if !as_it_stands(&right) => Some(slot.ident.clone()),
                        _ => None,
                    };
                    match late {
                        Some(named) => {
                            let by = self.gensym("side");
                            let set = Form::Write(by.clone(), Box::new(right));
                            // Where the name is looked for again, since
                            // the far side may be what first bound it.
                            let now = self.read(&named);
                            sequence(vec![set, prim_call(other, vec![now, Form::Read(by)])])
                        }
                        None => prim_call(other, vec![left, right]),
                    }
                }
            };
        }
        Ok(left)
    }

    fn comparison_head(&self) -> Option<(Prim, u32, usize)> {
        let t = self.table;
        let first = &self.look().lexeme;
        if t.spells("ext.op.in.negated", first) && t.spells("ext.op.in", &self.glance(1).lexeme) {
            let membership = t.dyadic.get(&self.glance(1).lexeme)?;
            return Some((Prim::Absent, membership.level, 2));
        }
        let binary = t.dyadic.get(first)?;
        match binary.prim {
            Prim::Selfsame if t.spells("ext.op.identical.negated", &self.glance(1).lexeme) => Some((Prim::Unlike, binary.level, 2)),
            Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::Selfsame | Prim::Unlike | Prim::Contains | Prim::Absent => Some((binary.prim, binary.level, 1)),
            _ => None,
        }
    }

    /// The next link runs beneath the true arm of this one. Its near
    /// side is the cell this link filled, so no middle is read twice.
    fn comparison_tail(&mut self, near: Address, operation: Prim, level: u32, words: usize) -> Res<Form> {
        for _ in 0..words { self.advance(); }
        let side = self.expr(level + 1)?;
        let far = self.gensym("next");
        let put = Form::Write(far.clone(), Box::new(side));
        let test = prim_call(operation, vec![Form::Read(near), Form::Read(far.clone())]);
        let answer = match self.comparison_head() {
            Some((following, tier, width)) if tier == level => {
                let rest = self.comparison_tail(far.clone(), following, tier, width)?;
                self.choose(test, rest, constant(Value::Flag(false)))
            }
            _ => test,
        };
        let judged = self.gensym("judged");
        Ok(sequence(vec![put, Form::Write(judged.clone(), Box::new(answer)), Form::Forget(far), Form::Read(judged)]))
    }

    /// `++p` and `p--` over any place a write reaches: the place is
    /// read, stepped by one and written back, and what stands afterwards
    /// is the value after the step, or the one that was there before it
    /// where the step is written after the place.
    fn step_of(&mut self, target: Form, by: i64, gives_new: bool) -> Res<Form> {
        self.gensyms += 1;
        let cell = format!("#step{}", self.gensyms);
        let was_by = self.stepping.replace(by.abs());
        let (was_stood, was_stands) = match gives_new {
            true => (self.stood.take(), self.stands.replace(cell.clone())),
            false => (self.stood.replace(cell.clone()), self.stands.take()),
        };
        let op = Prim::Onward(by >= 0);
        let sign = self.look().clone();
        let done = self.write_into(target, false, Some(op), sign);
        self.stepping = was_by;
        self.stood = was_stood;
        self.stands = was_stands;
        let done = done?;
        Ok(sequence(vec![done, self.read(&cell)]))
    }

    /// A piece of an expression, with a step written before or after it
    /// where the language spells one.
    fn monadic_expr(&mut self) -> Res<Form> {
        if let Some(by) = self.step_by(&self.look().clone()) {
            self.advance();
            let target = self.monadic_expr()?;
            return self.step_of(target, by, true);
        }
        let piece = self.monadic_piece()?;
        if let Some(by) = self.step_by(&self.look().clone()) {
            self.advance();
            return self.step_of(piece, by, false);
        }
        Ok(piece)
    }

    fn quotation(&mut self) -> Res<Form> {
        let start = self.advance();
        if start.shape == Shape::ByteQuote {
            return Ok(constant(Value::Octets { cell: Rc::new(std::cell::RefCell::new(start.lexeme.chars().map(|c| c as u8).collect())), changeable: false,
                lead: Rc::from(self.table.strings("ext.system.bytes.repr")[0].as_str()) }));
        }
        if start.shape == Shape::CharacterRow {
            let numbers = start.lexeme.split_whitespace().map(|word| word.parse().unwrap()).collect();
            return Ok(constant(Value::characters(numbers)));
        }
        if start.shape == Shape::Quote { return Ok(constant(Value::text(&start.lexeme))); }
        if start.shape == Shape::Unheld { return Ok(prim_call(Prim::UnheldText, vec![constant(Value::text(&start.lexeme))])); }
        if start.shape != Shape::Woven {
            return Err(self.table.single("ext.lexical.string.amiss").unwrap_or("Invalid string literal").to_owned());
        }
        let mut result = constant(Value::text(""));
        loop {
            if self.look().shape == Shape::WovenEnd { self.advance(); break; }
            let piece = if self.look().shape == Shape::Field {
                let conversion = self.advance().lexeme;
                let value = self.expr(0)?;
                let spec = self.quotation()?;
                prim_call(Prim::RenderField, vec![value, spec, constant(Value::text(&conversion))])
            } else { self.quotation()? };
            result = prim_call(Prim::Join, vec![result, piece]);
        }
        Ok(result)
    }

    fn monadic_piece(&mut self) -> Res<Form> {
        let table = self.table;
        let t = self.look().clone();
        if self.reading_yield && table.spells("op.mul", &t.lexeme) {
            self.advance();
            let value = self.monadic_expr()?;
            return Ok(if table.flag("ext.stmt.yield.suspends") { self.scope_unrun("ext.stmt.yield.unsupported") } else { value });
        }
        if self.key("ext.op.await") {
            let class_expression = self.class_bindings.last().map_or(false, |(depth, _)| *depth >= self.layers.len().saturating_sub(1));
            if self.forbids_await || !self.layers.last().unwrap().permits_async || class_expression {
                let scope = if self.layers.len() == 1 || class_expression { "outside function" } else { "outside async function" };
                return Err(format!("SyntaxError: 'await' {scope}"));
            }
            if self.layers[0].permits_async && self.layers.iter().skip(1).all(|layer| layer.gathering_kind.is_some()) { self.top_coroutine = true; }
            let scope = self.layers.last_mut().unwrap();
            if scope.gathering_kind.is_some() { scope.async_walk_seen = true; }
            self.advance();
            let pending = self.monadic_expr()?;
            return Ok(prim_call(Prim::AwaitResult, vec![pending]));
        }
        if self.key("ext.stmt.yield") {
            let begins = self.pos;
            let forbidden = if self.in_class_body() || !self.layers.iter().skip(1).any(|s| s.holds == Holds::Every) { Some(if table.spells("ext.stmt.yield.from", &self.glance(1).lexeme) { "'yield from' outside function" } else { "'yield' outside function" }.to_owned()) }
                else { self.layers.last().and_then(|s| s.gathering_kind).map(|kind| format!("'yield' inside {kind}")) };
            self.advance();
            self.generator_seen = true;
            let previous_yield = std::mem::replace(&mut self.reading_yield, true);
            let from = self.key("ext.stmt.yield.from");
            if from { self.advance(); }
            if from && table.has_any("ext.builtin.exceptions.syntax") && self.on_any("ext.stmt.unpack.rest") {
                return Err(String::from("SyntaxError: invalid syntax"));
            }
            let at_end = |r: &Self| r.on_stmt_end() || r.exhausted() || r.look().shape == Shape::Close
                || r.on_any("syntax.group.close") || r.on_any("syntax.array.close");
            // A spread unpacks into the tuple a yield of several values
            // yields, as it does into any display; a spread standing
            // alone as the whole answer is refused below, as the
            // reference refuses it.
            let mut values: Vec<(Form, bool)> = Vec::new();
            let mut comma = false;
            let mut spread_met = false;
            let mut star_at = self.pos;
            if from || !at_end(self) {
                loop {
                    let star = !from && self.table.single("ext.op.tuple").is_some() && self.on_any("ext.stmt.unpack.rest");
                    if star { spread_met = true; star_at = self.pos; self.advance(); }
                    let part = self.expr(0)?;
                    values.push((if star {
                        prim_call(Prim::Apart, vec![prim_call(Prim::Partition(1, Some(0)), vec![part]), constant(Value::Small(0))])
                    } else { part }, star));
                    if from || !self.on_any("syntax.call.separator") { break; }
                    comma = true;
                    self.advance();
                    if at_end(self) { break; }
                }
            }
            self.reading_yield = previous_yield;
            if table.has_any("ext.builtin.exceptions.syntax") {
                // A delegated yield answers one expression only: a comma
                // past it parts no tuple of its, as a plain yield's does.
                if from && self.on_any("syntax.call.separator") {
                    return Err(String::from("SyntaxError: invalid syntax"));
                }
                if spread_met && !comma {
                    self.pos = star_at;
                    return Err(String::from("SyntaxError: can't use starred expression here"));
                }
                if let Some(complaint) = forbidden {
                    let last = &self.tokens[self.pos - 1];
                    self.range_end = Some((last.column + last.lexeme.chars().count(), last.row));
                    self.pos = begins;
                    return Err(format!("SyntaxError: {complaint}"));
                }
            }
            if table.flag("ext.stmt.yield.suspends") {
                let value = if comma {
                    if spread_met {
                        let mut pieces = values.into_iter();
                        let (first_part, first_star) = pieces.next().expect("a first value");
                        let mut whole = if first_star { first_part } else { prim_call(Prim::MakeArray, vec![first_part]) };
                        for (part, star) in pieces {
                            let portion = if star { part } else { prim_call(Prim::MakeArray, vec![part]) };
                            whole = prim_call(Prim::TupleJoined, vec![whole, portion]);
                        }
                        if self.table.has_any("ext.builtin.tuple") { prim_call(Prim::Tupling, vec![whole]) } else { whole }
                    } else {
                        prim_call(Prim::MakeTuple, values.into_iter().map(|(part, _)| part).collect())
                    }
                } else { values.pop().map(|(part, _)| part).unwrap_or_else(|| constant(Value::Nil)) };
                return Ok(prim_call(if from { Prim::Delegate } else { Prim::Suspend }, vec![value]));
            }
            return Ok(prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.stmt.yield.unrun").unwrap_or_default()))]));
        }
        if table.spells("ext.stmt.class.special.declined", &t.lexeme) {
            self.advance();
            return self.subscript(constant(Value::Refusal(Rc::from(t.lexeme.as_str()))));
        }
        if table.spells("ext.stmt.class.special.stop", &t.lexeme) && !table.spells("ext.builtin.exceptions", &t.lexeme) {
            self.advance();
            let plan = crate::data::Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None,
                name: t.lexeme.clone(), under: None, methods: vec![], shared: std::cell::RefCell::new(vec![]),
                fields: vec![], constants: vec![], reaches: vec![], answers: vec![], sealed: std::cell::Cell::new(false),
            };
            return self.subscript(constant(Value::Blueprint(Rc::new(plan))));
        }
        if t.shape != Shape::Quote && table.spells("ext.literal.ellipsis", &t.lexeme) {
            self.advance();
            return self.subscript(constant(Value::Ellipsis));
        }
        // The value with which a method declines an operation, by name.
        if t.shape != Shape::Quote && table.spells("ext.literal.unimplemented", &t.lexeme) {
            self.advance();
            return self.subscript(constant(Value::Refusal(Rc::from(t.lexeme.as_str()))));
        }
        // The lambda word must stand bare: quoted, it is text and no
        // more, as when it is one of print's arguments.
        if t.shape == Shape::Bare && table.spells("ext.op.lambda", &t.lexeme) {
            self.advance();
            return self.lambda_form();
        }
        // `list($a, $b) = v`: the places named on the left each take
        // the matching place of the value on the right.
        if table.single("ext.op.tuple").is_none() && table.spells("ext.stmt.unpack", &t.lexeme) && matches!(t.shape, Shape::Sign | Shape::Bare) {
            self.advance();
            let places = self.pos;
            self.step_past_call()?;
            let sign = self.advance();
            if !table.strings("stmt.assign").iter().any(|w| *w == sign.lexeme) {
                return Err(format!("A taking-apart must be written on the left of a write, not '{}'", sign.lexeme));
            }
            let worth = self.expr(0)?;
            self.gensyms += 1;
            let holding = format!("#taken{}", self.gensyms);
            let kept = self.write(&holding, worth);
            let after = self.pos;
            self.pos = places;
            let mut steps = vec![kept];
            steps.extend(self.taken_apart(&holding)?);
            self.pos = after;
            steps.push(self.read(&holding));
            return Ok(sequence(steps));
        }
        // `(int) x`: a kind's word written within the grouping marks
        // before a value makes the value that kind. Only a word the
        // language names a kind by counts, so grouping a plain name is
        // still grouping.
        if table.flag("ext.op.cast") {
            let opens = table.single("syntax.group.open").map(str::to_string);
            let closes = table.single("syntax.group.close").map(str::to_string);
            if let (Some(open), Some(close)) = (opens, closes) {
                let made = self
                    .sign(&open)
                    .then(|| kind_made(table, &self.glance(1).lexeme))
                    .flatten()
                    .filter(|_| self.glance(2).shape == Shape::Sign && self.glance(2).lexeme == close);
                if let Some(made) = made {
                    self.advance();
                    self.advance();
                    self.advance();
                    let tier = table.monadic.values().map(|m| m.level).max().unwrap_or(0);
                    let inner = self.expr(tier)?;
                    return Ok(prim_call(made, vec![inner]));
                }
            }
        }
        if matches!(t.shape, Shape::Sign | Shape::Bare) {
            if let Some(op) = table.monadic.get(&t.lexeme).copied() {
                self.advance();
                let operand = self.expr(op.level)?;
                return Ok(prim_call(op.prim, vec![operand]));
            }
            if table.spells("ext.op.name_by_value", &t.lexeme) {
                // `$$a` and `${e}`: the name is whatever the value
                // spells. Only the outermost bindings keep names the
                // run can still see, so a name worked out inside a unit
                // of its own is turned down rather than quietly meaning
                // some other binding.
                if self.layers.len() > 1 {
                    return Err(format!("'{}' works a name out as the run goes, which only the outermost bindings keep", t.lexeme));
                }
                self.advance();
                let spells = self.spelling()?;
                return self.subscript(Form::Called(Box::new(spells)));
            }
            if table.spells("ext.op.hush", &t.lexeme) {
                // What the piece under the mark has to say about itself
                // goes unsaid; the value it comes to is unchanged.
                let tier = table.precedence.get(&t.lexeme).copied().unwrap_or(0);
                self.advance();
                let quiet = self.expr(tier)?;
                return Ok(Form::Silenced(Box::new(quiet)));
            }
            if t.shape == Shape::Sign && table.spells("ext.op.plus", &t.lexeme) {
                // A plus sign leaves its operand as it is, bound like a negation.
                self.advance();
                let tier = table.monadic.values().map(|m| m.level).max().unwrap_or(0);
                let held = self.expr(tier)?;
                return Ok(if table.has_any("ext.op.plus.non_number") { prim_call(Prim::NumberAlone, vec![held]) }
                    else if !table.strings("ext.lexical.number.imaginary").is_empty() { prim_call(Prim::Positive, vec![held]) } else { held });
            }
        }
        let node = match t.shape {
            Shape::Numeral => {
                self.advance();
                constant(numeral(&t.lexeme, table)?)
            }
            Shape::ByteQuote | Shape::Quote | Shape::CharacterRow | Shape::Woven | Shape::Unheld => {
                let mut text = self.quotation()?;
                if table.flag("ext.lexical.string.adjacent") {
                    while matches!(self.look().shape, Shape::ByteQuote | Shape::Quote | Shape::CharacterRow | Shape::Woven | Shape::Unheld) {
                        if (t.shape == Shape::ByteQuote) != (self.look().shape == Shape::ByteQuote) {
                            return Err(table.single("ext.lexical.string.bytes.mixed").unwrap_or("").to_owned());
                        }
                        text = prim_call(if t.shape == Shape::ByteQuote { Prim::Plus } else { Prim::Join }, vec![text, self.quotation()?]);
                    }
                }
                text
            }
            Shape::Bare if table.spells("ext.stmt.class.new", &t.lexeme) => {
                self.advance();
                let named = self.need_word("as the class to make")?;
                let stands = self.read_class(&named)?;
                let mut given = vec![self.class_reference(stands)?];
                if table.single("syntax.call.open").map_or(false, |o| self.sign(o)) {
                    self.advance();
                    // A maker takes its arguments as any other routine
                    // does, so one of its parameters that takes a cell
                    // is handed one.
                    let maker = table.single("ext.stmt.class.constructor").unwrap_or_default().to_string();
                    given.extend(self.arguments_of(&maker, "syntax.call.close", "syntax.call.separator")?);
                }
                prim_call(Prim::Spawn, given)
            }
            Shape::Bare if table.flag("ext.stmt.class.this.explicit") && table.spells("ext.stmt.class.parent", &t.lexeme)
                && self.uses_bound_callable(&t.lexeme) => {
                self.advance();
                let target = self.read(&t.lexeme);
                if table.single("syntax.call.open").is_some_and(|open| self.sign(open)) {
                    self.advance();
                    let arguments = self.arguments_of(&t.lexeme, "syntax.call.close", "syntax.call.separator")?;
                    invoke(target, arguments)
                } else { target }
            }
            // Beyond every class body the parent word the program has
            // bound, not opening a call, is an ordinary name, to be read,
            // listed or handed on. Unbound, it stands for the parent call
            // itself, as the arm below hands it over.
            Shape::Bare if table.flag("ext.stmt.class.this.explicit") && table.spells("ext.stmt.class.parent", &t.lexeme)
                && self.within.is_none()
                && (self.named_in_program.iter().any(|word| word == &t.lexeme)
                    || self.glance(1).shape == Shape::Sign
                        && (table.spells("stmt.assign", &self.glance(1).lexeme) || table.compound.contains_key(&self.glance(1).lexeme)))
                && table.single("syntax.call.open").map_or(true, |open| !(self.glance(1).shape == Shape::Sign && self.glance(1).lexeme == open)) => {
                self.advance();
                self.read(&t.lexeme)
            }
            Shape::Bare if table.has_any("ext.stmt.class.detail.root") && table.spells("ext.stmt.class.parent",&t.lexeme)
                && table.single("syntax.call.open").map_or(false,|open|self.glance(1).lexeme!=open) => {
                self.advance();
                constant(Value::Wrapped(9, PARENT_PAYLOAD.with(|value| value.clone())))
            }
            Shape::Bare if table.flag("ext.stmt.class.this.explicit") && table.spells("ext.stmt.class.parent", &t.lexeme) => {
                self.advance();
                let open = table.single("syntax.call.open").ok_or("A parent call needs brackets")?;
                if !self.sign(open) {
                    let unavailable = self.class_not_ready();
                    return self.subscript(unavailable);
                }
                self.need_sign(open, "after the parent word")?;
                let extra = self.args("syntax.call.close", "syntax.call.separator")?;
                let base = self.within.as_ref().map(|(n,b)| if table.has_any("ext.stmt.class.detail.root") {n.clone()} else {b.clone().unwrap_or_default()});
                let sign = table.single("ext.op.member").filter(|m| self.sign(m));
                match (extra.is_empty(), base, self.receiver.clone(), sign) {
                    (true, Some(_), Some(receiver), None) if !self.under_way.is_empty() => {
                        self.parts().needs_class_cell = true;
                        let private = self.parts().completed_class.ident.to_string();
                        let args = vec![self.read(&private), self.read(&receiver)];
                        let parent_word = constant(Value::Wrapped(9, PARENT_PAYLOAD.with(Rc::clone)));
                        invoke(parent_word, args)
                    }
                    (true, Some(base), Some(receiver), Some(_)) => {
                        self.advance();
                        let called = self.need_word("as the parent's member")?;
                        let parent = if table.has_any("ext.stmt.class.detail.root") {constant(Value::text(&base))} else {self.read(&base)};
                        let mut given = vec![self.read(&receiver), parent, constant(Value::text(&called))];
                        if self.sign(open) {
                            self.advance();
                            given.extend(self.args("syntax.call.close", "syntax.call.separator")?);
                            prim_call(Prim::Bid, given)
                        } else { self.class_not_ready() }
                    }
                    _ => self.class_not_ready(),
                }
            }
            Shape::Bare if !self.in_class_body() && !self.under_way.is_empty()
                && t.lexeme == "__classdict__" && table.has_any("ext.stmt.class.detail.kind") => {
                self.advance();
                let book = self.parts().book.clone().expect("class namespace prepared");
                self.read(&book.ident.to_string())
            }
            Shape::Bare if !self.in_class_body() && !self.under_way.is_empty()
                && table.spells("ext.stmt.class.detail.kind", &t.lexeme)
                && !self.layers.last().unwrap().idents.contains(&t.lexeme)
                && !self.gather_names.iter().any(|pair| pair.0 == t.lexeme) => {
                self.advance();
                self.parts().needs_class_cell = true;
                let hidden = self.parts().completed_class.ident.to_string();
                self.read(&hidden)
            }
            Shape::Bare if table.spells("ext.stmt.class.self", &t.lexeme) || table.spells("ext.stmt.class.parent", &t.lexeme) => {
                self.advance();
                self.read_class(&t.lexeme)?
            }
            Shape::Bare => {
                self.advance();
                if table.spells("literal.true", &t.lexeme) {
                    constant(Value::Flag(true))
                } else if table.spells("literal.false", &t.lexeme) {
                    constant(Value::Flag(false))
                } else if table.spells("literal.null", &t.lexeme) {
                    constant(Value::Nil)
                } else if self.place_depth == 0 && table.spells("ext.builtin.print.file.output", &t.lexeme) {
                    constant(Value::Channel(1))
                } else if self.place_depth == 0 && table.spells("ext.builtin.print.file.error", &t.lexeme) {
                    constant(Value::Channel(2))
                } else if matches!(table.prims.get(&t.lexeme), Some(Prim::Octets(0 | 1)))
                    && self.place_depth == 0 && !self.uses_bound_callable(&t.lexeme)
                    && !table.single("syntax.call.open").map_or(false, |o| self.sign(o)) {
                    let words = table.strings("ext.system.bytes.type");
                    constant(Value::OctetKind { changeable: table.prims.get(&t.lexeme) == Some(&Prim::Octets(1)),
                        shown: Rc::from(format!("{}{}{}", words[0], t.lexeme, words[1])) })
                } else if table.strings("ext.stmt.function.short").first().map_or(false, |word| word == &t.lexeme)
                    && (table.flag("ext.syntax.call.bind_names") || table.single("syntax.call.open").map_or(false, |o| self.sign(o)))
                {
                    // A routine written short is one expression, and
                    // takes with it every name standing around it: it
                    // has nowhere to say which of them it wants.
                    let built = self.short_func()?;
                    return self.subscript(built);
                } else if table.spells("stmt.function", &t.lexeme)
                    && table.single("syntax.call.open").map_or(false, |o| {
                        self.sign(o)
                            || (table.single("ext.op.reference").map_or(false, |m| self.sign(m)) && self.glance(1).shape == Shape::Sign && self.glance(1).lexeme == o)
                    })
                {
                    // A routine written where a value stands is bound to
                    // no name and stands for itself. It reaches none of
                    // the names around it, only the outermost ones, as a
                    // routine written out does.
                    let gives_cell = self.skip_reference();
                    self.giving_cells.push(gives_cell);
                    let built = self.func(ANONYMOUS.to_string(), false);
                    self.giving_cells.pop();
                    return self.subscript(built?);
                } else if table.single("syntax.call.open").map_or(false, |o| self.sign(o)) {
                    self.advance();
                    if table.prims.get(&t.lexeme) == Some(&Prim::Erase) {
                        return self.forget();
                    }
                    if table.prims.get(&t.lexeme) == Some(&Prim::Hollow) {
                        return self.hollow();
                    }
                    if table.prims.get(&t.lexeme) == Some(&Prim::Standing) {
                        return self.standing();
                    }
                    if table.prims.get(&t.lexeme) == Some(&Prim::Gather) {
                        // array(...) gathers what it is given, like a literal.
                        let items = self.elements("syntax.call.close", "syntax.call.separator")?;
                        return self.subscript(prim_call(Prim::MakeArray, items));
                    }
                    let mut args = self.arguments_of(&t.lexeme, "syntax.call.close", "syntax.call.separator")?;
                    if table.prims.get(&t.lexeme) == Some(&Prim::Define) {
                        // define("NAME", v) binds the global NAME here; its value is true.
                        let (Some(Form::Const(Value::Text(name))), 2) = (args.first(), args.len()) else {
                            return Err(format!("{}() needs a quoted name and a value", t.lexeme));
                        };
                        let name = name.to_string();
                        let value = args.pop().unwrap();
                        // A name carrying the scope mark spells one of a
                        // class's own values and no constant at all. A
                        // language with words for that turns the name
                        // down, saying them where the run reaches the
                        // call, so a program may take it as any fault.
                        let scoped = table.single("ext.op.scope").map_or(false, |m| name.contains(m));
                        match (scoped, table.single("ext.builtin.define.class_constant")) {
                            (true, Some(said)) => sequence(vec![value, prim_call(Prim::Raise, vec![constant(Value::text(said))])]),
                            _ => {
                                let slot = self.global_address(&name);
                                sequence(vec![Form::Write(slot, Box::new(value)), constant(Value::Flag(true))])
                            }
                        }
                    } else if matches!(table.prims.get(&t.lexeme), Some(Prim::Octets(_))) && self.arg_names.contains_key(&t.lexeme) {
                        let function = self.read(&t.lexeme);
                        invoke(function, args)
                    } else {
                        self.named_call(&t.lexeme, args)?
                    }
                } else if table.spells("ext.system.globals", &t.lexeme)
                    && table.single("op.index.open").map_or(false, |o| self.sign(o))
                {
                    // A word standing for all the outermost bindings
                    // taken as an array: a place in it is the binding
                    // whose name that place spells, which is how a
                    // language reaches a global from within a routine.
                    self.advance();
                    let spells = self.expr(0)?;
                    self.need_sign(table.single("op.index.close").unwrap(), "after the name of the binding")?;
                    return self.subscript(Form::Called(Box::new(spells)));
                } else if table.single("ext.op.scope").map_or(false, |m| self.sign(m)) {
                    // A name written before the scope mark names a
                    // class, so it is read as one.
                    self.read_class(&t.lexeme)?
                } else if table.flag("ext.syntax.call.bare")
                    && matches!(table.prims.get(&t.lexeme), Some(Prim::Bring) | Some(Prim::BringOnce))
                {
                    // A source read in stands where a value does as much
                    // as where a statement does, brackets or none:
                    // `return include $p`. The whole of what follows is
                    // its own, nothing written after binding looser.
                    let named = self.expr(0)?;
                    self.named_call(&t.lexeme, vec![named])?
                } else {
                    self.read(&t.lexeme)
                }
            }
            Shape::Sign => {
                if table.single("syntax.group.open") == Some(t.lexeme.as_str()) {
                    self.advance();
                    let inner = if self.reading_yield {
                        let mut values = Vec::new();
                        let mut tuple = self.on_any("syntax.group.close");
                        while !self.on_any("syntax.group.close") {
                            values.push(self.expr(0)?);
                            if !self.on_any("syntax.call.separator") { break; }
                            tuple = true;
                            self.advance();
                        }
                        let value = if tuple { prim_call(if table.flag("ext.stmt.yield.suspends") { Prim::MakeTuple } else { Prim::MakeArray }, values) } else { values.pop().unwrap() };
                        self.need_sign(table.single("syntax.group.close").unwrap(), "to close a group")?;
                        value
                    } else {
                        match self.ahead_in_item("ext.op.comprehension.for") {
                        Some(at) => self.generator_comprehension(at, table.single("syntax.group.close").unwrap())?,
                        None => {
                            let expression = if !table.has_any("ext.op.tuple") { self.expr(0)? }
                                else if self.on_any("syntax.group.close") {
                                if table.has_any("ext.builtin.tuple") { constant(Value::Tuple(Rc::new(Vec::new()))) } else { self.scope_unrun("ext.system.scope.unready") }
                            }
                                else { self.comma_value()? };
                            if table.has_any("ext.builtin.exceptions.syntax")
                                && matches!(self.look().shape, Shape::Bare | Shape::Numeral) {
                                return Err(String::from("SyntaxError: invalid syntax. Perhaps you forgot a comma?"));
                            }
                            self.need_sign(table.single("syntax.group.close").unwrap(), "to close a group")?;
                            expression
                        }
                        }
                    };
                    self.called_on_value(inner)?
                } else if table.single("syntax.array.open") == Some(t.lexeme.as_str()) {
                    self.advance();
                    if table.has_any("ext.op.comprehension.for") || table.has_any("ext.syntax.array.spread") {
                        self.gathered_literal("array")?
                    } else {
                        let items = self.elements("syntax.array.close", "syntax.array.separator")?;
                    prim_call(Prim::MakeArray, items)
                    }
                } else if table.single("syntax.map.open") == Some(t.lexeme.as_str()) {
                    self.advance();
                    if table.has_any("ext.op.comprehension.for") || table.has_any("ext.syntax.map.spread") || table.flag("ext.syntax.set") {
                        self.gathered_literal("map")?
                    } else {
                        let items = self.elements("syntax.map.close", "syntax.map.separator")?;
                    prim_call(Prim::MakeMap, items)
                    }
                } else {
                    // A language with words of its own for what stopped
                    // the reading puts them first; the kernel's plainer
                    // ones serve where the definition gives none.
                    return Err(match table.single("ext.system.reading.unexpected") {
                        Some(opening) => format!("{} {}", opening, t.lexeme),
                        None => format!("Unexpected token: {}", t.lexeme),
                    });
                }
            }
            _ => return Err("Expected an expression".to_string()),
        };
        self.subscript(node)
    }

    /// `unset(a, b[k])`: each name is left as though nothing were ever
    /// written to it, and each place named is taken out of its array.
    fn forget(&mut self) -> Res<Form> {
        self.forget_list(true, false, None)
    }

    fn forget_list(&mut self, bracketed: bool, targets: bool, end: Option<String>) -> Res<Form> {
        let table = self.table;
        let close = end.unwrap_or_else(|| table.single("syntax.call.close").unwrap().to_string());
        let sep = table.single("syntax.call.separator").map(str::to_string);
        let mut items = Vec::new();
        while if bracketed { !self.sign(&close) } else { !self.on_stmt_end() && !self.exhausted() && self.look().shape != Shape::Close } {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            if targets && table.has_any("ext.builtin.exceptions.syntax") {
                if self.look().shape == Shape::Bare && self.glance(1).lexeme == "[" && self.glance(2).lexeme == "*"
                    && self.glance(3).lexeme == "(" {
                    if let Some(closed) = self.pair_close(self.pos + 3, self.tokens.len()) {
                        if self.tokens[self.pos + 4..closed].iter().any(|word| word.lexeme == ":") {
                            return Err(String::from("SyntaxError: Invalid star expression"));
                        }
                    }
                }
                let token = self.look();
                let description = match token.lexeme.as_str() {
                    "None" | "False" | "True" | "__debug__" => Some(token.lexeme.as_str()),
                    "*" => Some("starred"),
                    "not" | "~" | "-" | "+" => Some("expression"),
                    _ if matches!(token.shape, Shape::Quote | Shape::Numeral | Shape::ByteQuote) => Some("literal"),
                    _ => None,
                };
                if let Some(description) = description { return Err(format!("SyntaxError: cannot delete {description}")); }
            }
            if targets && !self.bracketed_holder() {
                let closes = if self.on_any("syntax.group.open") { table.single("syntax.group.close") }
                    else if self.on_any("syntax.array.open") { table.single("syntax.array.close") } else { None };
                if let Some(closes) = closes {
                    let parentheses = self.on_any("syntax.group.open");
                    self.advance();
                    if table.has_any("ext.builtin.exceptions.syntax") && parentheses && self.sign("*") {
                        let comma = self.tokens[self.pos..].iter().take_while(|part| part.lexeme != closes).any(|part| part.lexeme == ",");
                        if !comma { return Err(String::from("SyntaxError: cannot use starred expression here")); }
                    }
                    items.push(self.forget_list(true, true, Some(closes.to_string()))?);
                    if self.on_any("syntax.call.separator") { self.advance(); }
                    continue;
                }
            }
            self.unsupported_place = false;
            let named = if targets {
                let place = self.deletion_place()?;
                self.called_on_value(place)?
            } else { self.expr(0)? };
            if targets && table.has_any("ext.builtin.exceptions.syntax") {
                if matches!(&named, Form::Apply(Callee::Code(_), _)) {
                    return Err(String::from("SyntaxError: cannot delete function call"));
                }
                let word = &self.look().lexeme;
                let invalid = match word.as_str() {
                    ":=" => Some("named expression"),
                    "if" => Some("conditional expression"),
                    _ if table.dyadic.contains_key(word) => Some("expression"),
                    _ => None,
                };
                if let Some(kind) = invalid { return Err(format!("SyntaxError: cannot delete {kind}")); }
            }
            items.push(if self.unsupported_place {
                prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.stmt.del.unrun").unwrap_or_default()))])
            } else { match named {
                Form::Read(slot) => {
                    // A bare name deleted was written, never used, as
                    // the reference counts it against a later `global`.
                    if targets && table.flag("ext.stmt.function.closes_over") {
                        let own = self.address_to_write(&slot.ident);
                        self.unmet_last_read(&slot.ident);
                        sequence(vec![Form::Read(own.clone()), Form::Forget(own)])
                    } else if targets { sequence(vec![self.read(&slot.ident), Form::Forget(slot)]) }
                    else { Form::Forget(slot) }
                },
                Form::Apply(Callee::Prim(Prim::At, _), mut args) if args.len() == 2 => {
                    let at = args.pop().unwrap();
                    match args.pop().unwrap() {
                        // Where a list lives in the cell its name stands
                        // for, the place is taken out of the list as it
                        // lies, and a walk still under way over it sees
                        // the shortening; elsewhere the name is written
                        // afresh with what is left.
                        Form::Read(slot) if self.table.has_any("ext.stmt.del") => {
                            let cell = self.cell_of(Form::Read(slot))?;
                            Form::ForgetWithin(Box::new(cell), Box::new(at))
                        }
                        Form::Read(slot) => {
                            let held = slot.ident.to_string();
                            let array = self.read(&held);
                            let left = prim_call(Prim::Erase, vec![array, at]);
                            self.write(&held, left)
                        }
                        // `unset($o->p[k])`, `unset(C::$a[k])`: what
                        // holds the place is asked for its own cell,
                        // and the place taken out of what it holds.
                        under => {
                            let cell = self.cell_of(under)?;
                            Form::ForgetWithin(Box::new(cell), Box::new(at))
                        }
                    }
                }
                // `unset($o->p)`: the property is taken off the thing
                // itself, which every name for it sees at once.
                Form::Apply(Callee::Prim(Prim::Of, _), args) if args.len() == 2 => prim_call(Prim::Pluck, args),
                // `unset($$x)`: the name is worked out as the run goes
                // and the binding it spells left standing for nothing.
                Form::Called(spells) => Form::ForgetCalled(spells),
                _ => return Err("Only a name, a place in an array or a property can be forgotten".to_string()),
            } });
            if let Some(s) = &sep {
                if self.sign(s) {
                    self.advance();
                }
            }
        }
        if bracketed { self.advance(); }
        items.push(constant(Value::Nil));
        Ok(sequence(items))
    }

    /// Whether what stands after the member mark is a value and not a
    /// word written out: a variable, or a piece within the block marks.
    fn member_named_by_value(&mut self, member: bool) -> bool {
        if self.table.single("block.open").map_or(false, |open| self.sign(open)) {
            return true;
        }
        if self.table.spells("ext.op.name_by_value", &self.look().lexeme) {
            return true;
        }
        // A bare variable names a member of a thing by what it keeps,
        // but written after the mark reaching into a class it names
        // that class's own value outright, mark and all. Only the mark
        // saying a value spells a name serves there.
        // Unless a call follows: `C::$m()` calls the method whose name
        // the binding holds, where `C::$m` is that class's own value of
        // the name.
        let here = self.look();
        let a_binding = here.shape == Shape::Bare && self.table.letter("identifier.variable_prefix").map_or(false, |mark| here.lexeme.starts_with(mark));
        let a_call = self.table.single("syntax.call.open").map_or(false, |open| { let next = self.glance(1); next.shape == Shape::Sign && next.lexeme == open });
        a_binding && (member || a_call)
    }

    /// The value spelling a member's name: a piece within the block
    /// marks, or a bare variable — bare, since a call bracket after it
    /// opens the method's arguments and not a call of the variable.
    fn member_value_name(&mut self, member: bool) -> Res<Form> {
        // After the mark reaching into a class, the mark is the one a
        // class's own values are written with, and what follows spells
        // the name outright: `C::$$n` is the value named by what `$n`
        // keeps. After the mark reaching into a thing there is no such
        // mark in the writing, so one standing there says the piece
        // spells a name and the member is named by what *that* binding
        // keeps: `$o->${e}` is a step further in.
        if self.table.spells("ext.op.name_by_value", &self.look().lexeme) {
            self.advance();
            let spells = self.spelling()?;
            return Ok(match member {
                true => Form::Called(Box::new(spells)),
                false => spells,
            });
        }
        let opens = self.table.single("block.open").map(str::to_string);
        let closes = self.table.single("block.close").map(str::to_string);
        if let (Some(open), Some(close)) = (opens, closes) {
            if self.sign(&open) {
                self.advance();
                let named = self.expr(0)?;
                self.need_sign(&close, "after the member's name")?;
                return Ok(named);
            }
        }
        let named = self.advance().lexeme;
        Ok(self.read(&named))
    }

    /// What comes after the mark saying a value spells a name: a piece
    /// written within the block marks, or else whatever binds as
    /// tightly as a negation, so `$$$a` is read from the inside out.
    fn spelling(&mut self) -> Res<Form> {
        let table = self.table;
        let opens = table.single("block.open").map(str::to_string);
        let closes = table.single("block.close").map(str::to_string);
        if let (Some(open), Some(close)) = (opens, closes) {
            if self.sign(&open) {
                self.advance();
                let named = self.expr(0)?;
                self.need_sign(&close, "after the name to work out")?;
                return Ok(named);
            }
        }
        let tier = table.monadic.values().map(|m| m.level).max().unwrap_or(0);
        self.expr(tier)
    }

    /// Step past a bracketed piece without reading it, so what follows
    /// may be read first: the places a taking-apart names are read only
    /// once the value they take from is worked out.
    fn step_past_call(&mut self) -> Res<()> {
        let open = self.table.single("syntax.call.open").ok_or("A taking-apart needs the call brackets")?.to_string();
        let close = self.table.single("syntax.call.close").ok_or("A taking-apart needs the call brackets")?.to_string();
        self.need_sign(&open, "after the word that takes a value apart")?;
        let mut deep = 1usize;
        while deep > 0 {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            if self.sign(&open) {
                deep += 1;
            } else if self.sign(&close) {
                deep -= 1;
            }
            self.advance();
        }
        Ok(())
    }

    /// A cell for something already built: a name, a place in an array
    /// or a property. Anything else is left as it stands, which serves
    /// for reading though not for writing back.
    fn cell_of(&mut self, form: Form) -> Res<Form> {
        Ok(match form {
            Form::Read(slot) => {
                // A name that closes over is shared where it was read: a
                // module name met inside a function names the module's
                // binding, and a fresh local spelt the same would hold
                // nothing and be nothing to take a place out of.
                let shared = if self.table.flag("ext.stmt.function.closes_over") { slot } else { self.address_to_write(&slot.ident.to_string()) };
                Form::Share(shared)
            }
            Form::Apply(Callee::Prim(Prim::Of, _), mut args) if args.len() == 2 => {
                let named = args.pop().expect("the property");
                let thing = args.pop().expect("what holds it");
                match named {
                    Form::Const(Value::Text(called)) => Form::ShareField(Box::new(thing), called),
                    _ => return Err("Only a property named outright has a cell to share".to_string()),
                }
            }
            Form::Apply(Callee::Prim(Prim::At, _), mut args) if args.len() == 2 => {
                let place = args.pop().expect("the place");
                let stands_on = args.pop().expect("what holds it");
                self.shared_at(place, stands_on)?
            }
            Form::Apply(Callee::Prim(Prim::Within, _), mut args) if args.len() == 2 => {
                let named = args.pop().expect("the value's name");
                let class = args.pop().expect("the class");
                match named {
                    Form::Const(Value::Text(called)) => Form::ShareOwn(Box::new(class), called),
                    _ => return Err("Only a class's own value named outright has a cell to share".to_string()),
                }
            }
            Form::Called(spells) => Form::ShareCalled(spells),
            other => other,
        })
    }

    /// The cell of the place a chain of looks names: the keys are taken
    /// apart from the innermost out, so each is worked out once and in
    /// order. Asking a place for its cell is a write as much as a read,
    /// so a name holding nothing yet is not complained about where a
    /// write makes what it needs.
    fn shared_at(&mut self, place: Form, stands_on: Form) -> Res<Form> {
        let mut keys = vec![place];
        let mut walk = stands_on;
        loop {
            match walk {
                Form::Apply(Callee::Prim(Prim::At, _), mut inner) if inner.len() == 2 => {
                    keys.push(inner.pop().expect("the place"));
                    walk = inner.pop().expect("what holds it");
                }
                Form::Read(slot) => {
                    keys.reverse();
                    let held = self.address_to_read(&slot.ident.to_string());
                    return Ok(Form::Muted(Box::new(Form::SharePlace(held, keys))));
                }
                // `&$o->p[k]`: the chain stands on something that is
                // no binding of its own, so that footing is asked for
                // its cell and the place taken from within it.
                held @ (Form::Apply(Callee::Prim(Prim::Of | Prim::Within, _), _) | Form::Called(_)) => {
                    keys.reverse();
                    let under = self.cell_of(held)?;
                    return Ok(Form::Muted(Box::new(Form::ShareWithin(Box::new(under), keys))));
                }
                _ => return Err("Only a place in a named array has a cell to share".to_string()),
            }
        }
    }

    /// What stands after the mark that shares a cell: a name, a place in
    /// an array, or a property. Whichever it is, a cell is made of it
    /// where it is not one already, so a name may be tied to it.
    fn a_shared_cell(&mut self, unshared: &[String], as_it_runs: bool, handed: Option<(String, usize, String)>) -> Res<Form> {
        // Where the asking stands, so that words said about it name that
        // line and not one a call along the way left behind.
        let row = (self.look().row as u32).saturating_sub(self.before);
        // Read as any expression is, a write among them: what is asked
        // to share a cell may be a write, and a write is not a place, so
        // its value is handed over and the language says so.
        let read = self.expr_at(0, true)?;
        Ok(match read {
            Form::Read(slot) => {
                let shared = self.address_to_write(&slot.ident.to_string());
                Form::Share(shared)
            }
            Form::Apply(Callee::Prim(Prim::Of, _), mut args) if args.len() == 2 => {
                let named = args.pop().expect("the property");
                let thing = args.pop().expect("what holds it");
                let Form::Const(Value::Text(called)) = named else {
                    return Err("Only a property named outright has a cell to share".to_string());
                };
                Form::ShareField(Box::new(thing), called)
            }
            Form::Apply(Callee::Prim(Prim::At, _), mut args) if args.len() == 2 => {
                let place = args.pop().expect("the place");
                let stands_on = args.pop().expect("what holds it");
                self.shared_at(place, stands_on)?
            }
            // A class's own value is kept once for the whole class, and
            // so keeps a cell as a binding does.
            Form::Apply(Callee::Prim(Prim::Within, _), mut args) if args.len() == 2 => {
                let named = args.pop().expect("the value's name");
                let class = args.pop().expect("the class");
                let Form::Const(Value::Text(called)) = named else {
                    return Err("Only a class's own value named outright has a cell to share".to_string());
                };
                Form::ShareOwn(Box::new(class), called)
            }
            // A binding named as the run goes has a cell as any binding
            // does, and asking for it asks for that very one.
            Form::Called(spells) => Form::ShareCalled(spells),
            // A write that ties one name to another's cell has that very
            // cell to hand over: the write is done and the cell it made
            // is what goes over, not what it holds.
            Form::Apply(Callee::Prim(Prim::Seq, word), mut parts)
                if handed.is_some() && matches!(parts.first(), Some(Form::Tie(..))) =>
            {
                let tied = match parts.first() {
                    Some(Form::Tie(held, _)) => held.clone(),
                    _ => unreachable!("a tie stands first"),
                };
                parts.pop();
                parts.push(Form::Share(tied));
                Form::Apply(Callee::Prim(Prim::Seq, word), parts)
            }
            // Anything else is read as it stands: a call of a routine
            // giving back a cell answers with one already, and what has
            // no cell to share is written plainly, which is what a
            // language asking to share one from something without one
            // does rather than stopping. Where a language has words for
            // that, they are said once the value is worked out.
            found => {
                // A method is written as any other routine is, so one
                // written to give back a cell gives one however it is
                // called: through a thing, or through a class. Which
                // argument names it depends on which of the two.
                let named_at = |op: &Prim| match op {
                    Prim::Ask => Some(1),
                    Prim::Bid => Some(2),
                    _ => None,
                };
                let shares = match &found {
                    Form::Apply(Callee::Code(target), _) => matches!(target.as_ref(), Form::Read(slot) if self.gives_back.contains(slot.ident.as_ref())),
                    Form::Apply(Callee::Prim(op, _), given) => match named_at(op).and_then(|at| given.get(at)) {
                        Some(Form::Const(Value::Text(called))) => self.gives_back.contains(called.as_ref()),
                        _ => false,
                    },
                    _ => false,
                };
                // A call answering with a value where a cell was asked
                // for is a thing a language may only remark upon; a
                // value that was never going to have one — a literal, a
                // write — it refuses outright, naming the parameter.
                // Only the run tells them apart, since a write fastening
                // one name to another's cell answers with that cell.
                if let Some((called, which, spelt)) = &handed {
                    // A run of forms one after another is no call, however
                    // it is spelt inside.
                    let calls = match &found {
                        Form::Apply(Callee::Code(_), _) => true,
                        Form::Apply(Callee::Prim(op, _), _) => !matches!(op, Prim::Seq),
                        _ => false,
                    };
                    return Ok(match (calls, unshared.first()) {
                        (true, Some(said)) => Form::CellOrSaid(Some("notice"), Rc::from(said.as_str()), row, Box::new(found)),
                        (true, None) => found,
                        (false, _) => {
                            let told = format!("{}(): Argument #{} ({}) could not be passed by reference", called, which + 1, spelt);
                            Form::CellOrSaid(None, Rc::from(told.as_str()), row, Box::new(found))
                        }
                    });
                }
                // Whether a name may be fastened to what a call answers
                // with is settled by how the routine is written, so it
                // is known here. Whether a routine giving back a cell
                // was given one to give is settled by the run, the value
                // itself saying whether it is a cell.
                match (unshared.first(), as_it_runs, shares) {
                    (Some(said), true, _) => Form::HeldEither("notice", Rc::from(said.as_str()), row, Box::new(found)),
                    (Some(said), false, false) => {
                        self.gensyms += 1;
                        let held = format!("#shared{}", self.gensyms);
                        let kept = self.write(&held, found);
                        let told = Form::Remark("notice", Rc::from(said.as_str()), row);
                        sequence(vec![kept, told, self.read(&held)])
                    }
                    _ => found,
                }
            }
        })
    }

    /// `list($a, , $b) = v`: each place named takes the matching place
    /// of the value. A place left out is stepped over and still counts,
    /// and a taking-apart within one takes the place holding it.
    fn taken_apart(&mut self, holding: &str) -> Res<Vec<Form>> {
        let table = self.table;
        let open = table.single("syntax.call.open").unwrap().to_string();
        let close = table.single("syntax.call.close").unwrap().to_string();
        let sep = table.single("syntax.call.separator").map(str::to_string);
        self.need_sign(&open, "after the word that takes a value apart")?;
        let mut steps = Vec::new();
        let mut at = 0usize;
        while !self.sign(&close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            let stepped = sep.as_ref().map_or(false, |s| self.sign(s)) || self.sign(&close);
            if !stepped {
                self.gensyms += 1;
                let held = format!("#place{}", self.gensyms);
                let from = self.read(holding);
                let there = prim_call(Prim::Apart, vec![from, constant(Value::Small(at as i64))]);
                steps.push(self.write(&held, there));
                if table.spells("ext.stmt.unpack", &self.look().lexeme) {
                    self.advance();
                    steps.extend(self.taken_apart(&held)?);
                } else {
                    let target = self.expr_at(0, false)?;
                    let was = self.waiting.replace(held);
                    let stood = self.look().clone();
                    let done = self.write_into(target, false, None, stood);
                    self.waiting = was;
                    steps.push(done?);
                }
            }
            at += 1;
            if let Some(s) = &sep {
                if self.sign(s) {
                    self.advance();
                    continue;
                }
            }
            break;
        }
        self.need_sign(&close, "after the places to take apart")?;
        Ok(steps)
    }

    /// `isset(a, b[k])`: whether every one of them is something other
    /// than nothing. A name never written and a place an array does not
    /// hold both count as nothing, and neither is complained about, so
    /// every look inside is a glance and the whole is muted.
    /// `empty(x)`: whether what the name or place holds is untrue,
    /// asked as gently as asking whether it is there at all, since a
    /// place that is not there holds nothing and nothing is untrue.
    fn hollow(&mut self) -> Res<Form> {
        let close = self.table.single("syntax.call.close").unwrap().to_string();
        let one = self.expr(0)?;
        self.need_sign(&close, "after what is asked about")?;
        Ok(prim_call(Prim::Hollow, vec![Form::Muted(Box::new(glancing(one)))]))
    }

    fn standing(&mut self) -> Res<Form> {
        let table = self.table;
        let close = table.single("syntax.call.close").unwrap().to_string();
        let sep = table.single("syntax.call.separator").map(str::to_string);
        let mut asked = Vec::new();
        while !self.sign(&close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            let one = self.expr(0)?;
            asked.push(Form::Muted(Box::new(glancing(one))));
            if let Some(s) = &sep {
                if self.sign(s) {
                    self.advance();
                }
            }
        }
        self.advance();
        if asked.is_empty() {
            return Err("Nothing was asked about".to_string());
        }
        Ok(prim_call(Prim::Standing, asked))
    }

    /// The name a class is bound under. Where a language lets a class
    /// go by its name however the name is written, every class binds
    /// its name written small, so that each way of writing it arrives.
    fn class_binding(&self, name: &str) -> String {
        if self.table.flag("ext.stmt.class.this.explicit") { return name.to_owned(); }
        // A binding written where a class is named stays a binding, and
        // bindings are told apart by how they are written; only a
        // class's own name is bound however it is written.
        let sigil = self.table.letter("identifier.variable_prefix");
        if sigil.map_or(false, |mark| name.starts_with(mark)) {
            return name.to_string();
        }
        let folded = match self.table.flag("ext.system.class.folded") {
            true => name.to_lowercase(),
            false => name.to_string(),
        };
        format!("{}{}", folded, crate::form::OF_A_CLASS)
    }

    /// What is left of a class named by a value: a property of a thing,
    /// a place in an array, one after another. A call that follows the
    /// chain is the maker's and never the chain's, so no step of it is
    /// ever a call.
    fn class_reference(&mut self, mut node: Form) -> Res<Form> {
        let table = self.table;
        if let Some(mark) = table.single("ext.op.member") {
            while self.sign(mark) {
                self.advance();
                let named = self.need_word("after the member mark")?;
                node = prim_call(Prim::Of, vec![node, constant(Value::text(&named))]);
            }
        }
        let (Some(open), Some(close)) = (table.single("op.index.open"), table.single("op.index.close")) else { return Ok(node) };
        while self.sign(open) {
            self.advance();
            let index = self.expr(0)?;
            self.need_sign(close, "after the place naming the class")?;
            node = prim_call(Prim::At, vec![node, index]);
        }
        Ok(node)
    }

    /// The class a name stands for: `self` names the class being read
    /// and `parent` the one it is built on.
    fn read_class(&mut self, name: &str) -> Res<Form> {
        let table = self.table;
        if table.spells("ext.stmt.class.self", name) {
            let (here, _) = self.within.clone().ok_or_else(|| format!("'{}' belongs inside a class", name))?;
            let bound = self.class_binding(&here);
            return Ok(self.read_bound(&here, &bound));
        }
        if table.spells("ext.stmt.class.parent", name) {
            let (here, under) = self.within.clone().ok_or_else(|| format!("'{}' belongs inside a class", name))?;
            let under = under.ok_or_else(|| format!("Class {} is built on nothing", here))?;
            let bound = self.class_binding(&under);
            return Ok(self.read_bound(&under, &bound));
        }
        let bound = self.class_binding(name);
        Ok(self.read_bound(name, &bound))
    }

    /// Reads what a class is bound as, under the name it was written
    /// with: what a class is bound as is the kernel's own doing, so a
    /// fault speaks of the name and never of the binding behind it.
    fn read_bound(&mut self, written: &str, bound: &str) -> Form {
        let slot = self.address_to_read(bound);
        Form::Read(Address { ident: Rc::from(written), ..slot })
    }

    /// `thing->member` and `class::member`, in a chain.
    fn members(&mut self, mut node: Form) -> Res<Form> {
        let table = self.table;
        loop {
            if table.has_any("ext.op.lambda") { node = self.called_on_value(node)?; }
            let reaching = table.single("ext.op.member").map_or(false, |m| self.sign(m));
            let owning = table.single("ext.op.scope").map_or(false, |m| self.sign(m));
            if !reaching && !owning {
                return Ok(node);
            }
            self.advance();
            // A value may stand where a member's name stands: the member
            // is the one that value spells, worked out as the run goes.
            // Nothing else changes — the name is already an argument of
            // the reading or the calling, so a value serves where a word
            // written out would.
            if table.flag("ext.op.member.by_value") && self.member_named_by_value(!owning) {
                let spells = self.member_value_name(!owning)?;
                let calling = table.single("syntax.call.open").map_or(false, |o| self.sign(o));
                if owning {
                    if !calling {
                        node = prim_call(Prim::Within, vec![node, spells]);
                        continue;
                    }
                    // A method is given the thing it is for, so what the
                    // class was written as comes after it, and the name
                    // the value spells after that.
                    let this = match (&self.within, table.single("ext.stmt.class.this")) {
                        (Some(_), Some(this)) => self.read(&this.to_string()),
                        _ => constant(Value::Nil),
                    };
                    let mut given = vec![this, node, spells];
                    self.advance();
                    given.extend(self.arguments_of("the method", "syntax.call.close", "syntax.call.separator")?);
                    node = prim_call(Prim::Bid, given);
                    continue;
                }
                let mut given = vec![node, spells];
                if calling {
                    self.advance();
                    given.extend(self.arguments_of("the method", "syntax.call.close", "syntax.call.separator")?);
                }
                node = match calling {
                    false => prim_call(Prim::Of, given),
                    true => prim_call(Prim::Ask, given),
                };
                continue;
            }
            let named = self.need_word("after the member mark")?;
            let calling = table.single("syntax.call.open").map_or(false, |o| self.sign(o));
            if calling && table.strings("ext.builtin.method.from_number").iter().any(|word| word.rsplit('.').next() == Some(named.as_str())) {
                self.advance();
                let values = self.args("syntax.call.close", "syntax.call.separator")?;
                node = invoke(prim_call(Prim::Of, vec![node, constant(Value::text(&named))]), values);
                continue;
            }
            let kind_follows = self.kind_mark.map_or(false, |mark| mark >= self.pos
                && self.tokens[self.pos..mark].iter().all(|token| token.shape == Shape::Sign
                    && table.spells("syntax.group.close", &token.lexeme)));
            if reaching && table.flag("ext.op.member.pipes") && !table.spells("ext.text.format", &named) && (calling || self.place_depth == 0 && !self.on_writing() && !kind_follows) {
                let target = match &node { Form::Read(slot) => Some(slot.clone()), _ => None };
                let held = self.gensym("subject");
                let save = Form::Write(held.clone(), Box::new(node));
                let test = prim_call(Prim::HasMember, vec![Form::Read(held.clone()), constant(Value::text(&named))]);
                let begin = self.pos;
                let yes = self.limb(Traps::Naught, |r| {
                    let member = prim_call(Prim::Of, vec![Form::Read(held.clone()), constant(Value::text(&named))]);
                    if !calling { return Ok(member); }
                    r.advance();
                    let args = r.args("syntax.call.close", "syntax.call.separator")?;
                    Ok(invoke(member, args))
                })?;
                self.pos = begin;
                let no = self.limb(Traps::Naught, |r| {
                    let mut args = vec![Form::Read(held.clone())];
                    if calling {
                        r.advance();
                        args.extend(r.args("syntax.call.close", "syntax.call.separator")?);
                    }
                    if let Some((label, _)) = crate::table::BUILTIN_LABELS.iter().find(|(label, prim)| *prim == Prim::ValueMethod && table.spells(label, &named)) {
                        let receiver = args.remove(0);
                        let operation = label.strip_prefix("ext.builtin.method.").expect("method entry");
                        let member = prim_call(Prim::BindValueMethod, vec![receiver, constant(Value::text(operation))]);
                        return Ok(if calling { invoke(member, args) } else { member });
                    }
                    if table.flag("ext.stmt.yield.suspends") && ["ext.stmt.yield.send", "ext.stmt.yield.close", "ext.stmt.yield.throw"].iter().any(|key| table.spells(key, &named)) {
                        args.insert(1, constant(Value::text(&named)));
                        return Ok(prim_call(Prim::Ask, args));
                    }
                    if let Some(op @ Prim::SetCall(1..=17)) = table.prims.get(&named).copied() {
                        if calling { return Ok(Form::Apply(Callee::Prim(op, Rc::from(named.as_str())), args)); }
                        // Named without a call, a set's method is read
                        // as a member of the set, where the table words
                        // what an absent member says; a table without
                        // those words keeps the old refusal.
                        if table.strings("ext.builtin.method.error.attribute").len() == 3 {
                            return Ok(prim_call(Prim::Of, vec![args.remove(0), constant(Value::text(&named))]));
                        }
                        return Ok(prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.builtin.set.method.unavailable").unwrap_or_default()))]));
                    }
                    if !calling && matches!(table.prims.get(&named), Some(Prim::Octets(14))) {
                        return Ok(prim_call(Prim::Of, vec![args.remove(0), constant(Value::text(&named))]));
                    }
                    if !calling && matches!(table.prims.get(&named), Some(Prim::Octets(_))) {
                        args.push(prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.system.bytes.unready").unwrap_or("")))]));
                        return Ok(sequence(args));
                    }
                    // The receiver's kind answers to no such name, and
                    // a table wording that complaint has no pipe to fall
                    // into: the value is told it has no such member,
                    // whatever a name of that spelling holds.
                    if table.prims.get(&named).is_none() && table.strings("ext.builtin.method.error.attribute").len() == 3 {
                        return Ok(prim_call(Prim::Of, vec![Form::Read(held.clone()), constant(Value::text(&named))]));
                    }
                    let fallback = r.named_call(&named, args)?;
                    if matches!(table.prims.get(&named), Some(Prim::Append | Prim::Replace)) {
                        return Ok(match &target {
                            Some(slot) => sequence(vec![fallback, Form::Write(slot.clone(), Box::new(Form::Read(held.clone()))), constant(Value::Nil)]),
                            None => r.class_not_ready(),
                        });
                    }
                    Ok(fallback)
                })?;
                let branch = self.choose(test, yes, no);
                // The subject is forgotten once the member is reached or
                // called: what it holds must go when the program lets go.
                let reached = self.gensym("reached");
                node = sequence(vec![save, Form::Write(reached.clone(), Box::new(branch)), Form::Forget(held), Form::Release(reached)]);
                continue;
            }
            let mut given = vec![node];
            if owning && !calling && table.spells("ext.stmt.class", &named) {
                node = prim_call(Prim::Named, given);
                continue;
            }
            if owning && calling {
                // A method is given the thing it is for, so what the class
                // was written as comes second.
                let this = match (&self.within, table.single("ext.stmt.class.this")) {
                    (Some(_), Some(this)) => self.read(&this.to_string()),
                    _ => constant(Value::Nil),
                };
                given.insert(0, this);
            }
            let bare = match (owning, table.letter("identifier.variable_prefix")) {
                (true, Some(sigil)) => named.trim_start_matches(sigil).to_string(),
                _ => named,
            };
            given.push(constant(Value::text(&bare)));
            if calling {
                self.advance();
                given.extend(self.arguments_of(&bare, "syntax.call.close", "syntax.call.separator")?);
            }
            node = match (owning, calling) {
                (false, false) => prim_call(Prim::Of, given),
                (false, true) => prim_call(Prim::Ask, given),
                (true, false) => prim_call(Prim::Within, given),
                (true, true) => prim_call(Prim::Bid, given),
            };
        }
    }

    /// A value standing in a group may be called straight off: `(f)(x)`,
    /// and one call after another, `(f)(x)(y)`.
    fn called_on_value(&mut self, mut node: Form) -> Res<Form> {
        let Some(open) = self.table.single("syntax.call.open").map(str::to_string) else {
            return Ok(node);
        };
        while self.sign(&open) {
            self.advance();
            let given = self.args("syntax.call.close", "syntax.call.separator")?;
            node = Form::Apply(Callee::Code(Box::new(node)), given);
        }
        Ok(node)
    }

    fn bracket_part(&mut self, close: &str, comma: Option<&str>) -> Res<Form> {
        if self.on_any("ext.syntax.array.spread") {
            self.advance();
            self.expr(0)?;
            return Ok(self.scope_unrun("ext.op.index.spread.unsupported"));
        }
        // The elision mark is read as the value the table gives it,
        // where it gives one, and refused where it gives none.
        if !self.table.has_any("ext.literal.ellipsis") && self.table.strings("ext.op.index.slice.ellipsis").iter().any(|word| self.sign(word)) {
            self.advance();
            return Ok(prim_call(Prim::SliceRefused, Vec::new()));
        }
        let separators = self.table.strings("ext.op.index.slice").to_vec();
        let mut parts = Vec::new();
        let mut spanning = false;
        loop {
            let at_mark = separators.iter().any(|word| self.sign(word));
            let at_end = self.sign(close) || comma.map_or(false, |word| self.sign(word));
            parts.push(if at_mark || (spanning && at_end) { constant(Value::Nil) } else { self.expr(0)? });
            if parts.len() == 3 || !separators.iter().any(|word| self.sign(word)) { break; }
            spanning = true;
            self.advance();
        }
        // A fourth part is no slice at all, and the table may say how
        // the reading stops over it.
        if spanning && parts.len() == 3 && separators.iter().any(|word| self.sign(word)) {
            if let Some(amiss) = self.table.single("ext.op.index.slice.amiss").filter(|word| !word.is_empty()) {
                return Err(amiss.to_owned());
            }
        }
        Ok(if spanning {
            parts.resize_with(3, || constant(Value::Nil));
            prim_call(Prim::SliceBounds, parts)
        } else {
            parts.pop().expect("the single place")
        })
    }

    fn subscript(&mut self, mut node: Form) -> Res<Form> {
        if self.table.flag("ext.syntax.call.chained") || self.table.has_any("ext.op.lambda") {
            node = self.called_on_value(node)?;
        }
        node = self.members(node)?;
        let (Some(open), Some(close)) = (self.table.single("op.index.open"), self.table.single("op.index.close")) else { return Ok(node) };
        while self.sign(open) {
            // `a[]`: the place after the last, which only a store reaches.
            if self.table.flag("ext.op.index.append") && self.glance(1).shape == Shape::Sign && self.glance(1).lexeme == close {
                self.pos += 2;
                node = prim_call(Prim::AtEnd, vec![node]);
                continue;
            }
            self.advance();
            if self.table.has_any("ext.builtin.exceptions.syntax") {
                let begin = if self.sign(":") && self.glance(1).lexeme == "(" { Some(self.pos + 1) }
                    else if self.sign("(") { Some(self.pos) } else { None };
                if let Some(start) = begin {
                    if let Some(stop) = self.pair_close(start, self.tokens.len()) {
                        let spread = self.tokens.get(start + 1).is_some_and(|part| part.lexeme == "*");
                        let has_comma = self.tokens[start..stop].iter().any(|part| part.lexeme == ",");
                        let has_colon = start != self.pos || self.tokens.get(stop + 1).is_some_and(|part| part.lexeme == ":");
                        if spread && !has_comma && has_colon { return Err(String::from("SyntaxError: cannot use starred expression here")); }
                    }
                }
            }
            if self.table.has_any("ext.builtin.exceptions.syntax") && self.sign("*") {
                let next = self.glance(1).lexeme.as_str();
                let incomplete = next == close || next == ":";
                let slice_in_group = next == "(" && self.tokens[self.pos + 2..].iter()
                    .take_while(|token| token.lexeme != ")")
                    .any(|token| token.lexeme == ":");
                if incomplete || slice_in_group { return Err(String::from("SyntaxError: Invalid star expression")); }
            }
            let separator = self.table.single("syntax.call.separator");
            let mut keys = vec![self.bracket_part(close, separator)?];
            let several = self.table.has_any("ext.op.index.slice") && separator.map_or(false, |word| self.sign(word));
            if several {
                while separator.map_or(false, |word| self.sign(word)) {
                    self.advance();
                    if self.sign(close) { break; }
                    keys.push(self.bracket_part(close, separator)?);
                }
            }
            let key = match (several, self.table.has_any("ext.builtin.slice")) {
                (true, true) => prim_call(Prim::MakeTuple, keys),
                (true, false) => prim_call(Prim::SliceRefused, keys),
                _ => keys.pop().expect("one key"),
            };
            self.need_sign(close, "after array index")?;
            node = prim_call(Prim::At, vec![node, key]);
            // What a look comes to may itself be called.
            if self.table.single("syntax.call.open").map_or(false, |o| self.sign(o)) {
                self.advance();
                let args = self.args("syntax.call.close", "syntax.call.separator")?;
                node = invoke(node, args);
            }
            node = self.members(node)?;
        }
        if self.table.flag("ext.syntax.call.chained") && self.on_any("syntax.call.open") {
            return self.subscript(node);
        }
        Ok(node)
    }

    /// The elements of a literal: as `args` reads them, except that
    /// `k => v` becomes one coupled value.
    fn gather_name(&mut self, stem: &str) -> String {
        self.gensym(stem).ident.to_string()
    }

    /// Find a word at this level, before an item ends. Quoted text is
    /// never a mark, however it happens to be spelled.
    fn ahead_in_item(&self, label: &str) -> Option<usize> {
        let table = self.table;
        let mut nesting = Vec::new();
        let pairs = [("syntax.group.open", "syntax.group.close"), ("syntax.array.open", "syntax.array.close"), ("syntax.map.open", "syntax.map.close")];
        for index in self.pos..self.tokens.len() {
            let token = &self.tokens[index];
            if !matches!(token.shape, Shape::Bare | Shape::Sign) { continue; }
            let text = token.lexeme.as_str();
            if nesting.is_empty() {
                if table.spells(label, text) {
                    let begins = if label == "ext.op.comprehension.for" && index > self.pos && table.spells("ext.op.comprehension.async", &self.tokens[index - 1].lexeme) { index - 1 } else { index };
                    return Some(begins);
                }
                if table.spells("syntax.call.separator", text) { return None; }
            }
            if let Some((_, end)) = pairs.iter().find(|(start, _)| table.spells(start, text)) {
                nesting.push(*end);
            } else if pairs.iter().any(|(_, end)| table.spells(end, text)) {
                match nesting.pop() {
                    Some(end) if table.spells(end, text) => (),
                    _ => return None,
                }
            }
        }
        None
    }

    fn gathered_literal(&mut self, family: &str) -> Res<Form> {
        let closing = self.table.single(&format!("syntax.{}.close", family)).unwrap().to_string();
        let separator = self.table.single(&format!("syntax.{}.separator", family)).unwrap().to_string();
        let mapped = family == "map" && (!self.table.flag("ext.syntax.set") || self.sign(&closing)
            || self.ahead_in_item("syntax.map.pair").is_some() || self.on_any("ext.syntax.map.spread"));
        if self.table.has_any("ext.builtin.exceptions.syntax") {
            let mut inner = Vec::new();
            let mut comma_before_for = false;
            for part in self.tokens.iter().skip(self.pos) {
                if inner.is_empty() && part.shape == Shape::Sign && part.lexeme == closing { break; }
                if inner.is_empty() {
                    if part.shape == Shape::Sign && part.lexeme == separator { comma_before_for = true; }
                    if part.shape == Shape::Bare && part.lexeme == "for" && comma_before_for {
                        return Err(String::from("SyntaxError: did you forget parentheses around the comprehension target?"));
                    }
                }
                if part.shape == Shape::Sign {
                    match part.lexeme.as_str() {
                        "(" | "[" | "{" => inner.push(part.lexeme.as_str()),
                        ")" | "]" | "}" => { inner.pop(); },
                        _ => (),
                    }
                }
            }
        }
        if let Some(next) = self.ahead_in_item("ext.op.comprehension.for") {
            return self.gather_comprehension(next, &closing, mapped);
        }
        let mut value = prim_call(if mapped { Prim::MakeMap } else if family == "map" { Prim::EmptySet } else { Prim::MakeArray }, vec![]);
        while !self.sign(&closing) {
            let spreading = self.on_any(if mapped { "ext.syntax.map.spread" } else { "ext.syntax.array.spread" });
            if spreading { self.advance(); }
            let mut beginning = self.pos;
            let mut item = self.expr(0)?;
            if mapped && !spreading {
                if self.table.has_any("ext.builtin.exceptions.syntax") && (self.sign(&closing) || self.sign(&separator)) {
                    return Err(String::from("SyntaxError: ':' expected after dictionary key"));
                }
                self.need_sign(self.table.single("syntax.map.pair").unwrap(), "between the key and its value")?;
                beginning = self.pos;
                if self.table.has_any("ext.builtin.exceptions.syntax") && (self.sign(&closing) || self.sign(&separator)) {
                    return Err(String::from("SyntaxError: expression expected after dictionary key and ':'"));
                }
                if self.table.has_any("ext.builtin.exceptions.syntax") && self.sign("*") {
                    return Err(String::from("SyntaxError: cannot use a starred expression in a dictionary value"));
                }
                let right = self.expr(0)?;
                item = prim_call(Prim::Couple, vec![item, right]);
            }
            if self.table.has_any("ext.builtin.exceptions.syntax") && self.look().lexeme == "fur" && self.glance(1).shape == Shape::Bare {
                return Err(String::from("SyntaxError: invalid syntax. Did you mean 'for'?"));
            }
            if self.table.has_any("ext.builtin.exceptions.syntax") && matches!(self.look().shape, Shape::Bare | Shape::Numeral | Shape::Quote | Shape::ByteQuote) {
                let t = self.look();
                let ending = if t.end_column == 0 { t.column + t.lexeme.chars().count() } else { t.end_column };
                self.range_end = Some((ending, t.end_row.max(t.row)));
                if self.tokens[beginning..self.pos].iter().all(|word| word.shape == Shape::Quote) { beginning = self.pos - 1; }
                self.pos = beginning;
                return Err(String::from("SyntaxError: invalid syntax. Perhaps you forgot a comma?"));
            }
            value = prim_call(Prim::ExtendLiteral(mapped, spreading), vec![value, item]);
            if self.sign(&closing) { break; }
            self.need_sign(&separator, "between parts of a literal")?;
        }
        self.advance();
        if family == "map" && !mapped && self.table.has_any("ext.stmt.class.special") { value = prim_call(Prim::DistinctObjects, vec![value]); }
        Ok(value)
    }

    fn generator_comprehension(&mut self, clause: usize, end: &str) -> Res<Form> {
        if !self.table.flag("ext.stmt.yield.suspends") { return self.gather_comprehension(clause, end, false); }
        let head = self.pos;
        self.pos = clause;
        self.pos = self.divided_at(self.pos, self.tokens.len(), "ext.op.comprehension.in").into_iter().next().ok_or(if self.table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: 'in' expected after for-loop variables" } else { "Expected a comprehension source" })? + 1;
        let begins = self.pos;
        let source = self.expr(1)?;
        let ends = self.pos;
        let parameter = self.gather_name("first_source");
        let previous = self.source_before.replace((begins, ends, parameter.clone()));
        let mut async_result = false;
        let routine = self.routine("<genexpr>", Holds::Every, Traps::Yields, vec![parameter], 1, |r| {
            r.layers.last_mut().unwrap().gathering_kind = Some("generator expression");
            r.pos = clause;
            let before = r.gather_names.len();
            r.reserve_gathering(clause)?;
            let body = r.gather_tail(head, "", false)?;
            r.gather_names.truncate(before);
            r.need_sign(end, "after a generator expression")?;
            r.reject_gathering_assignment("generator expression")?;
            async_result = r.layers.last().unwrap().async_walk_seen;
            r.generator_seen = true;
            Ok(body)
        })?;
        self.source_before = previous;
        let walk = if self.table.spells("ext.op.comprehension.async", &self.tokens[clause].lexeme) { Prim::AsyncWalked } else { Prim::Walked };
        let value = Form::Apply(Callee::Code(Box::new(routine)), vec![prim_call(walk, vec![source])]);
        if async_result && self.layers.len() == 1 && self.layers[0].permits_async { self.top_coroutine = true; }
        Ok(if async_result { prim_call(Prim::AsyncGathered, vec![value]) } else { value })
    }

    fn gather_comprehension(&mut self, first_for: usize, end: &str, dictionary: bool) -> Res<Form> {
        // Evaluate the outermost source before creating the inner routine.
        // Its yield, names and side effects belong to the surrounding body.
        // A class body is no closure, so a comprehension written
        // straight in one would see none of its names once its own
        // routine is pushed, where the reference reads the outermost
        // walk's own source in the body's own reading before that
        // routine is ever entered. That one source is read here, the
        // way `generator_comprehension` already reads a generator
        // expression's; what it comes to is handed to the
        // comprehension's own routine as its argument, and every
        // other clause and the expression read back run inside that
        // routine exactly as they did, seeing nothing of the body's
        // names, as a method does not either.
        if self.table.flag("ext.stmt.function.closes_over") && (self.table.has_any("ext.builtin.exceptions.syntax") || self.in_class_body()) {
            let entry = self.pos;
            self.pos = first_for;
            self.pos = self.divided_at(self.pos, self.tokens.len(), "ext.op.comprehension.in").into_iter().next().ok_or(if self.table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: 'in' expected after for-loop variables" } else { "Expected a comprehension source" })? + 1;
            let begins = self.pos;
            let source = self.expr(1)?;
            let bounds = self.span_since(begins);
            let ends = self.pos;
            let parameter = self.gather_name("first_source");
            let previous = self.source_before.replace((begins, ends, parameter.clone()));
            self.pos = entry;
            let routine = self.routine("<gathering>", Holds::Every, Traps::Yields, vec![parameter], 1,
                |reader| reader.gather_in_scope(first_for, end, dictionary))?;
            self.source_before = previous;
            if self.layers.len() == 1 && self.layers[0].permits_async && self.tokens[first_for..self.pos].iter().any(|t| self.table.spells("ext.op.comprehension.async", &t.lexeme) || self.table.spells("ext.op.await", &t.lexeme)) { self.top_coroutine = true; }
            let walks = self.table.flag("ext.stmt.yield.suspends");
            let begin = if self.table.spells("ext.op.comprehension.async", &self.tokens[first_for].lexeme) { Prim::AsyncWalked }
                else if walks { Prim::Walked } else { Prim::Iterated };
            return Ok(self.located(bounds, Form::Apply(Callee::Code(Box::new(routine)), vec![prim_call(begin, vec![source])])));
        }
        if self.table.flag("ext.stmt.function.closes_over") {
            let routine = self.routine("<gathering>", Holds::Every, Traps::Yields, Vec::new(), 0,
                |reader| reader.gather_in_scope(first_for, end, dictionary))?;
            return Ok(Form::Apply(Callee::Code(Box::new(routine)), Vec::new()));
        }
        self.gather_in_scope(first_for, end, dictionary)
    }

    fn gather_in_scope(&mut self, first_for: usize, end: &str, dictionary: bool) -> Res<Form> {
        self.layers.last_mut().unwrap().gathering_kind = Some(if dictionary { "dict comprehension" } else if end == "]" { "list comprehension" } else { "set comprehension" });
        let expression_at = self.pos;
        self.pos = first_for;
        let old_names = self.gather_names.len();
        self.reserve_gathering(first_for)?;
        let name = self.gather_name("gathered");
        let empty = prim_call(if dictionary { Prim::MakeMap } else if self.table.flag("ext.syntax.set") && self.table.spells("syntax.map.close", end) { Prim::EmptySet } else { Prim::MakeArray }, Vec::new());
        let start = self.write(&name, empty);
        let work = self.gather_tail(expression_at, &name, dictionary)?;
        self.gather_names.truncate(old_names);
        if self.table.has_any("ext.builtin.exceptions.syntax") && self.look().lexeme == "of" && self.glance(1).shape == Shape::Bare {
            return Err(String::from("SyntaxError: invalid syntax. Did you mean 'if'?"));
        }
        self.need_sign(end, "to finish a comprehension")?;
        self.reject_gathering_assignment(if dictionary { "dict comprehension" } else if end == "]" { "list comprehension" } else { "set comprehension" })?;
        let answer = self.read(&name);
        Ok(sequence(vec![start, work, answer]))
    }

    fn gathering_bindings(&mut self, span: std::ops::Range<usize>, first_binding: usize) -> Res<()> {
        let mut first = span.start;
        let limit = span.end;
        if first >= limit { return Ok(()); }
        let commas = self.divided_at(first, limit, "ext.op.tuple");
        if !commas.is_empty() {
            for boundary in commas.into_iter().chain(std::iter::once(limit)) {
                self.gathering_bindings(first..boundary, first_binding)?;
                first = boundary + 1;
            }
        } else if self.table.spells("ext.stmt.unpack.rest", &self.tokens[first].lexeme) {
            self.gathering_bindings(first + 1..limit, first_binding)?;
        } else {
            let token = &self.tokens[first];
            let family = ["syntax.group", "syntax.array"].into_iter().find(|family|
                token.shape == Shape::Sign && self.table.single(&format!("{family}.open")) == Some(token.lexeme.as_str()));
            if let Some(family) = family {
                let closing = self.table.single(&format!("{family}.close")).unwrap();
                let mut level = 1;
                let mut cursor = first + 1;
                while cursor < limit {
                    let next = &self.tokens[cursor];
                    if next.shape == Shape::Sign {
                        if next.lexeme == token.lexeme { level += 1; }
                        if next.lexeme == closing { level -= 1; }
                    }
                    cursor += 1;
                    if level == 0 { break; }
                }
                if cursor == limit && level == 0 { self.gathering_bindings(first + 1..limit - 1, first_binding)?; }
            } else if limit - first == 1 && token.shape == Shape::Bare {
                let word = token.lexeme.clone();
                if self.layers.last().unwrap().expression_targets.contains(&word) {
                    return Err(format!("SyntaxError: comprehension inner loop cannot rebind assignment expression target '{word}'"));
                }
                if self.gather_names.iter().skip(first_binding).any(|pair| pair.0 == word) { return Ok(()); }
                let binding = match self.table.flag("ext.stmt.function.closes_over") && self.table.has_any("ext.builtin.exceptions.syntax") {
                    true => word.to_string(),
                    false => self.gather_name("gather_binding"),
                };
                self.gather_names.push((word, binding.clone()));
                self.address_to_write(&binding);
            }
        }
        Ok(())
    }

    /// Build the clauses outside the expression they govern. Each walk
    /// owns its names; the first source still sees the names outside it.
    fn reject_gathering_assignment(&self, description: &str) -> Res<()> {
        if self.table.has_any("ext.builtin.exceptions.syntax") {
            if self.on_assign() {
                let hint = match description { "generator expression" => "", _ => " here. Maybe you meant '==' instead of '='?" };
                return Err(format!("SyntaxError: cannot assign to {description}{hint}"));
            }
            if self.look().shape == Shape::Sign && self.table.compound.contains_key(&self.look().lexeme) {
                return Err(format!("SyntaxError: '{description}' is an illegal expression for augmented assignment"));
            }
        }
        Ok(())
    }

    fn reserve_gathering(&mut self, start: usize) -> Res<()> {
        let own = self.gather_names.len();
        let mut cursor = start;
        let mut closing = Vec::new();
        while let Some(word) = self.tokens.get(cursor) {
            if word.shape == Shape::Finish { break; }
            if closing.is_empty() && word.shape == Shape::Bare && self.table.spells("ext.op.comprehension.for", &word.lexeme) {
                let end = self.divided_at(cursor + 1, self.tokens.len(), "ext.op.comprehension.in")
                    .into_iter().next().ok_or(if self.table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: 'in' expected after for-loop variables" } else { "Expected a comprehension source" })?;
                self.gathering_bindings(cursor + 1..end, own)?;
                cursor = end + 1;
                continue;
            }
            if word.shape == Shape::Sign {
                let pairs = [("syntax.group.open", "syntax.group.close"), ("syntax.array.open", "syntax.array.close"), ("syntax.map.open", "syntax.map.close")];
                if let Some((_, end)) = pairs.iter().find(|(begin, _)| self.table.spells(begin, &word.lexeme)) {
                    closing.push(*end);
                } else if pairs.iter().any(|(_, end)| self.table.spells(end, &word.lexeme)) && closing.pop().is_none() {
                    break;
                }
            }
            cursor += 1;
        }
        Ok(())
    }

    fn gather_tail(&mut self, expression_at: usize, answer: &str, dictionary: bool) -> Res<Form> {
        if self.on_any("ext.op.comprehension.if") {
            self.advance();
            let condition = self.expr(1)?;
            let accepted = self.gather_tail(expression_at, answer, dictionary)?;
            return Ok(self.choose(condition, accepted, constant(Value::Nil)));
        }
        if self.on_any("ext.op.comprehension.async") {
            self.layers.last_mut().unwrap().async_walk_seen = true;
            self.advance();
            if !self.on_any("ext.op.comprehension.for") { return Err("Expected a walk after the asynchronous word".into()); }
            let in_class = self.class_bindings.last().map_or(false, |(depth, _)| *depth >= self.layers.len().saturating_sub(1));
            if !answer.is_empty() && (!self.layers.last().unwrap().permits_async || in_class) {
                return Err(String::from("SyntaxError: asynchronous comprehension outside of an asynchronous function"));
            }
            return self.gather_tail(expression_at, answer, dictionary);
        }
        if !self.on_any("ext.op.comprehension.for") {
            let after_clauses = self.pos;
            self.pos = expression_at;
            let spread = self.on_any(if dictionary { "ext.syntax.map.spread" } else { "ext.syntax.array.spread" });
            if spread { self.advance(); }
            let previous = self.reading_yield;
            self.reading_yield |= answer.is_empty();
            let mut term = self.expr(0)?;
            self.reading_yield = previous;
            if dictionary && !spread {
                self.need_sign(self.table.single("syntax.map.pair").unwrap(), "in a map comprehension")?;
                let worth = self.expr(0)?;
                term = prim_call(Prim::Couple, vec![term, worth]);
            }
            if !self.on_any("ext.op.comprehension.for") && !self.on_any("ext.op.comprehension.async") { return Err("Expected a comprehension clause after its expression".into()); }
            self.pos = after_clauses;
            if answer.is_empty() { return Ok(prim_call(if spread { Prim::Delegate } else { Prim::Suspend }, vec![term])); }
            let so_far = self.read_taking(answer);
            let enlarged = prim_call(Prim::ExtendLiteral(dictionary, spread), vec![so_far, term]);
            return Ok(self.write(answer, enlarged));
        }
        let asynchronous = self.pos > 0 && self.table.spells("ext.op.comprehension.async", &self.tokens[self.pos - 1].lexeme);
        self.advance();
        let target_begin = self.pos;
        let target_stop = self.divided_at(target_begin, self.tokens.len(), "ext.op.comprehension.in")
            .into_iter().next().ok_or(if self.table.has_any("ext.builtin.exceptions.syntax") { "SyntaxError: 'in' expected after for-loop variables" } else { "Expected the word before a comprehension source" })?;
        if self.table.has_any("ext.builtin.exceptions.syntax") && self.has_added_target(target_begin..target_stop) {
            return Err(String::from("SyntaxError: cannot assign to expression"));
        }
        self.gathering_bindings(target_begin..target_stop, 0)?;
        self.pos = target_stop + 1;
        let walks = self.table.flag("ext.stmt.yield.suspends");
        let beginning = self.pos;
        let source = match self.source_before.clone().filter(|(at, _, _)| *at == self.pos) {
            Some((_, end, parameter)) => { self.pos = end; self.read(&parameter) }
            None => {
                let value = self.expr(1)?;
                prim_call(if asynchronous { Prim::AsyncWalked } else if walks { Prim::Walked } else { Prim::Iterated }, vec![value])
            }
        };
        let bounds = self.span_since(beginning);
        let source = self.located(bounds, source);
        let source_name = self.gather_name("gather_source");
        let hold = self.write(&source_name, source);
        let cursor = self.gather_name("gather_cursor");
        let begin = self.write(&cursor, constant(Value::Small(0)));
        let bag = self.read(&source_name);
        let index = self.read(&cursor);
        let test = if walks { prim_call(Prim::MoreYet, vec![bag, index]) }
            else { prim_call(Prim::Lt, vec![index, prim_call(Prim::Length, vec![bag])]) };
        let test = self.located(bounds, test);
        let bag = self.read(&source_name);
        let index = self.read(&cursor);
        let item = prim_call(if walks { Prim::AtHand } else { Prim::At }, vec![bag, index]);
        let item_name = self.gather_name("gather_item");
        let mut body = vec![self.write(&item_name, item)];
        let continue_at = self.pos;
        match self.distribute(target_begin..target_stop, &item_name) {
            Ok(binding) => body.push(binding),
            Err(error) => {
                let message = self.loop_target_error(target_begin..target_stop, error);
                if message.starts_with("SyntaxError: cannot assign to") { self.pos = target_begin; }
                return Err(message);
            }
        }
        self.pos = continue_at;
        body.push(self.gather_tail(expression_at, answer, dictionary)?);
        let before = self.read(&cursor);
        let onward = self.write(&cursor, prim_call(Prim::Plus, vec![before, constant(Value::Small(1))]));
        let cycle = Form::Cycle { test: Box::new(test), body: Box::new(sequence(body)), step: Some(Box::new(onward)), after: false, otherwise: None };
        Ok(sequence(vec![hold, begin, cycle]))
    }

    fn elements(&mut self, close_key: &str, sep_key: &str) -> Res<Vec<Form>> {
        let close = self.table.single(close_key).unwrap().to_string();
        let sep = self.table.single(sep_key).map(str::to_string);
        let mark = self.table.single("syntax.map.pair").map(str::to_string);
        let mut items = Vec::new();
        while !self.sign(&close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            // `[&$a]`: the place holds the binding's own cell, so a
            // write through either name is a write the other sees.
            let tied = self.table.single("ext.op.reference").map_or(false, |m| self.sign(m));
            let item = if tied {
                self.advance();
                let named = self.need_word("as the name to share a cell with")?;
                let slot = self.address_to_write(&named);
                Form::Share(slot)
            } else {
                self.expr(0)?
            };
            items.push(match &mark {
                Some(m) if self.sign(m) => {
                    self.advance();
                    let value = self.expr(0)?;
                    prim_call(Prim::Couple, vec![item, value])
                }
                _ => item,
            });
            if let Some(s) = &sep {
                if self.sign(s) {
                    self.advance();
                }
            }
        }
        self.advance();
        Ok(items)
    }

    /// The arguments of a call by name: where the program said the
    /// reference sign, the name's own cell goes rather than a copy of
    /// what it holds, so the caller sees what the program writes.
    fn arguments_of(&mut self, called: &str, close_key: &str, sep_key: &str) -> Res<Vec<Form>> {
        let shared = self.shared_args.get(called).cloned().unwrap_or_default();
        if !shared.iter().any(|x| *x) {
            return self.args(close_key, sep_key);
        }
        let close = self.table.single(close_key).unwrap().to_string();
        let sep = self.table.single(sep_key).map(str::to_string);
        let mut items = Vec::new();
        while !self.sign(&close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            // A parameter taking a cell is handed the cell of whatever
            // names one: a binding, a place in an array, a property.
            // What names none is handed its value, and the language says
            // so where it has words for it.
            if shared.get(items.len()).copied().unwrap_or(false) {
                let words = self.table.strings("ext.op.reference.unshared.handed").to_vec();
                let spelt = self.arg_names.get(called).and_then(|all| all.get(items.len())).cloned().unwrap_or_default();
                let which = items.len();
                items.push(self.a_shared_cell(&words, false, Some((called.to_string(), which, spelt)))?);
            } else {
                items.push(self.expr(0)?);
            }
            if let Some(s) = &sep {
                if self.sign(s) {
                    self.advance();
                }
            }
        }
        self.advance();
        Ok(items)
    }

    fn unparenthesized_generator(&mut self) -> Res<()> {
        let mut nesting = Vec::new();
        let mut begins = self.pos;
        let mut preceding = 0;
        let mut clause_seen = false;
        let mut binding_names = false;
        for at in self.pos..self.tokens.len() {
            let token = &self.tokens[at];
            if token.shape != Shape::Sign && token.shape != Shape::Bare { continue; }
            let word = token.lexeme.as_str();
            if nesting.is_empty() {
                if word == "=" && !(at == begins + 1 && self.tokens[begins].shape == Shape::Bare) {
                    if at > begins + 1 && self.table.has_any("ext.builtin.exceptions.syntax") {
                        let spreading = match self.tokens[begins].lexeme.as_str() {
                            "*" => Some("iterable"), "**" => Some("keyword"), _ => None,
                        };
                        if let Some(kind) = spreading { return Err(format!("SyntaxError: cannot assign to {kind} argument unpacking")); }
                    }
                    let strings = &self.tokens[begins..at.saturating_sub(1).max(begins)];
                    if !strings.is_empty() && strings.iter().all(|entry| entry.shape == Shape::Quote) {
                        self.pos = at - 2;
                        return Err(String::from("SyntaxError: invalid syntax. Perhaps you forgot a comma?"));
                    }
                    self.range_end = Some((token.column + 1, token.row));
                    self.pos = begins;
                    return Err(String::from("SyntaxError: expression cannot contain assignment, perhaps you meant \"==\"?"));
                }
                if word == ")" || (word == "," && !binding_names) {
                    if clause_seen && (preceding != 0 || word == ",") {
                        let last = &self.tokens[at - 1];
                        self.range_end = Some((last.column + last.lexeme.chars().count(), last.row));
                        self.pos = begins;
                        return Err("SyntaxError: Generator expression must be parenthesized".to_owned());
                    }
                    if word == ")" { return Ok(()); }
                    preceding += 1; begins = at + 1; clause_seen = false;
                }
                if self.table.spells("ext.op.comprehension.for", word) {
                    clause_seen = true;
                    binding_names = true;
                } else if self.table.spells("ext.op.comprehension.in", word) { binding_names = false; }
            }
            match word {
                "(" | "[" | "{" => nesting.push(word),
                ")" | "]" | "}" => { if nesting.pop().is_none() { break; } }
                _ => ()
            }
        }
        Ok(())
    }

    fn args(&mut self, close_key: &str, sep_key: &str) -> Res<Vec<Form>> {
        if self.table.has_any("ext.builtin.exceptions.syntax") { self.unparenthesized_generator()?; }
        if let Some(at) = self.ahead_in_item("ext.op.comprehension.for") {
            if self.table.has_any("ext.builtin.exceptions.syntax") && self.glance(1).shape == Shape::Sign && self.glance(1).lexeme == "=" {
                return Err(String::from("SyntaxError: invalid syntax. Maybe you meant '==' or ':=' instead of '='?"));
            }
            let end = self.table.single(close_key).unwrap().to_string();
            return Ok(vec![self.generator_comprehension(at, &end)?]);
        }
        let close = self.table.single(close_key).unwrap().to_string();
        let sep = self.table.single(sep_key).map(str::to_string);
        let mut items = Vec::new();
        let mut named_values = Vec::new();
        // The keywords written out in this call, so that one written
        // twice is refused as the text is read, as the reference
        // refuses it.
        let mut spelled: Vec<String> = Vec::new();
        // Once a keyword or a spread of pairs is written, nothing may
        // follow in order; the words for that come from this label.
        let misplaced = self.table.strings("ext.syntax.call.amiss.order").to_vec();
        let mut keyword_written = false;
        let mut pairs_written = false;
        while !self.sign(&close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            let label = self.look().shape == Shape::Bare && self.glance(1).shape == Shape::Sign
                && (self.table.spells("syntax.call.label", &self.glance(1).lexeme)
                    || (self.table.flag("ext.syntax.call.bind_names") && self.table.spells("stmt.assign", &self.glance(1).lexeme)));
            let mut tag = None;
            let bind = self.table.flag("ext.syntax.call.bind_names") && close_key == "syntax.call.close";
            if label {
                if bind {
                    let word = self.original_words[self.pos].lexeme.clone();
                    let twice = self.table.strings("ext.syntax.call.amiss.repeated");
                    if spelled.contains(&word) && !twice.is_empty() {
                        return Err(format!("{}{}{}", twice[0], word, twice.get(1).map_or("", String::as_str)));
                    }
                    if self.table.has_any("ext.builtin.exceptions.syntax")
                        && (word == "__debug__" || ["literal.true", "literal.false", "literal.null"]
                            .iter().any(|key| self.table.spells(key, &word))) {
                        return Err(format!("SyntaxError: cannot assign to {word}"));
                    }
                    spelled.push(word.clone());
                    tag = Some(Value::text(&word));
                }
                self.pos += 2;
            } else if bind && matches!(self.look().shape, Shape::Bare | Shape::Sign) {
                let word = &self.look().lexeme;
                if self.table.spells("ext.syntax.call.spread.pairs", word) { tag = Some(Value::Flag(true)); }
                else if self.table.spells("ext.syntax.call.spread", word) { tag = Some(Value::Flag(false)); }
                if tag.is_some() { self.advance(); }
            }
            if bind && matches!(tag, Some(Value::Flag(false)))
                && matches!(self.look().lexeme.as_str(), ")" | ":") {
                return Err(String::from("SyntaxError: Invalid star expression"));
            }
            if bind && misplaced.len() == 3 {
                match &tag {
                    Some(Value::Flag(false)) if pairs_written => return Err(misplaced[2].clone()),
                    None if pairs_written => return Err(misplaced[0].clone() + &misplaced[1]),
                    None if keyword_written => return Err(misplaced[0].clone()),
                    Some(Value::Text(_)) => keyword_written = true,
                    Some(Value::Flag(true)) => pairs_written = true,
                    _ => {}
                }
            }
            if self.table.has_any("ext.builtin.exceptions.syntax") && self.key("ext.stmt.yield") {
                return Err(String::from("SyntaxError: invalid syntax"));
            }
            if label && bind && self.table.has_any("ext.builtin.exceptions.syntax")
                && (self.sign(&close) || sep.as_ref().is_some_and(|mark| self.sign(mark))) {
                return Err(String::from("SyntaxError: expected argument value expression"));
            }
            let value = self.expr(0)?;
            let named = matches!(tag, Some(Value::Text(_) | Value::Flag(true)));
            let argument = match tag {
                Some(key) => prim_call(Prim::Couple, vec![constant(key), value]),
                None => value,
            };
            if named { named_values.push(argument); } else { items.push(argument); }
            if let Some(s) = &sep {
                if self.sign(s) {
                    self.advance();
                }
            }
        }
        self.advance();
        items.extend(named_values);
        Ok(items)
    }

    fn uses_bound_callable(&self, word: &str) -> bool {
        if !self.table.flag("ext.syntax.names.shadow_builtins") { return false; }
        let depth = self.layers.len();
        if self.in_class_body() {
            if let Some(body) = self.under_way.last() {
                if body.lexical_members.iter().any(|entry| entry == word) { return true; }
                if body.book.is_some() && !matches!(self.table.prims.get(word), Some(Prim::HereBook | Prim::MembersOf)) { return true; }
            }
        }
        let member = self.class_bindings.last().is_some_and(|(level, entries)| *level == depth && entries.contains_key(word));
        let declared = self.class_globals.last().is_some_and(|(level, words)| *level == depth && words.iter().any(|entry| entry == word));
        if member || declared || self.gather_names.iter().any(|entry| entry.0 == word) {
            return true;
        }
        for layer in self.layers.iter().skip(1).rev() {
            if layer.holds == Holds::Nothing { continue; }
            if layer.aliases.iter().any(|entry| entry.0 == word)
                || layer.borrowed.iter().any(|entry| entry == word)
                || layer.idents.iter().any(|entry| entry == word) {
                return true;
            }
            if layer.holds == Holds::Every && !self.table.flag("ext.stmt.function.closes_over") { break; }
        }
        !self.native_exports.contains(word) && self.named_in_program.iter().any(|entry| entry == word)
    }

    fn named_call(&mut self, name: &str, args: Vec<Form>) -> Res<Form> {
        // A name the program has bound is called as that name, in front
        // of any builtin word spelled the same, where the table says so.
        if self.uses_bound_callable(name) {
            let target = self.read(name);
            return Ok(invoke(target, args));
        }
        match self.table.prims.get(name).copied().filter(|op| !matches!(op, Prim::SetCall(1..=17))) {
            // A class body's `locals()` is CPython's own class
            // namespace, not a picture of it: a write through it
            // (`locals()[k] = v`) is seen by a later bare name read in
            // the same body, becomes a member of the class the body
            // forms, and a second `locals()` there answers with the
            // very dictionary the first one gave, `is` and all. `vars()`
            // with nothing handed to it asks the very same thing.
            // `class_book` makes that dictionary the first time either
            // is read, and answers with it every time after.
            Some(Prim::HereBook | Prim::MembersOf) if args.is_empty() && self.in_class_body() => Ok(self.class_book()),
            // `eval` read where a class body stands, handed no
            // dictionaries of its own, reads the body's own names the
            // way any other read there does, past the world beyond it
            // -- the same map `locals()` above is built from, handed
            // over as the near dictionary, with the outermost one
            // behind it as the far.
            Some(Prim::Weigh) if args.len() == 1 && self.in_class_body() => {
                let mut all = args;
                all.push(prim_call(Prim::WorldBook, Vec::new()));
                all.push(self.class_names_map());
                Ok(Form::Apply(Callee::Prim(Prim::Weigh, Rc::from(name)), all))
            }
            Some(Prim::Textual(work)) if work != crate::text::Work::REPR && !name.contains('.') => {
                let declared = self.read(name);
                Ok(invoke(declared, args))
            }
            // What is put in front of may be a name standing for a
            // shared cell, since the library of a language may spell it
            // as a routine taking one, so the place it names is looked
            // for while the run goes and not here.
            Some(Prim::Front) => Ok(Form::Apply(Callee::Prim(Prim::Front, Rc::from(name)), args)),
            Some(op @ (Prim::Append | Prim::Replace)) => {
                if !matches!(args.first(), Some(Form::Read(_))) {
                    return Err(format!("First argument to {}() must be an array variable name", name));
                }
                Ok(Form::Apply(Callee::Prim(op, Rc::from(name)), args))
            }
            Some(op) => Ok(Form::Apply(Callee::Prim(op, Rc::from(name)), args)),
            None => {
                let target = self.read(name);
                Ok(invoke(target, args))
            }
        }
    }

    // ---------- postfix

    /// Names up to a stop or the end, as statements and a symbolic stack.
    /// Where else they stop depends on what they are: a block's words at
    /// its end (a deeper indentation is an error), a program value's at
    /// its bracket (indentation passes), a line's at the line end.
    fn rpn_body(&mut self, stops: &[String], run: Mode) -> Res<(Vec<Form>, Vec<Form>)> {
        let mut stmts = Vec::new();
        let mut stack = Vec::new();
        loop {
            if run == Mode::Single {
                while self.look().shape == Shape::Sign && self.table.separates(&self.look().lexeme) {
                    self.advance();
                }
            } else {
                self.skip_line_ends();
            }
            if self.exhausted() || stops.iter().any(|s| self.lexeme_of(s)) {
                return Ok((stmts, stack));
            }
            let mark = self.look().shape;
            if matches!(mark, Shape::Open | Shape::Close | Shape::LineEnd) {
                if run == Mode::Quoted {
                    self.advance();
                    continue;
                }
                if run == Mode::Body && mark == Shape::Open {
                    return Err("Unexpected indentation".to_string());
                }
                return Ok((stmts, stack));
            }
            let t = self.advance();
            match t.shape {
                Shape::Numeral => stack.push(constant(numeral(&t.lexeme, self.table)?)),
                Shape::Quote => stack.push(constant(Value::text(&t.lexeme))),
                Shape::Quoted => {
                    self.skip_line_ends();
                    let taker = self.advance();
                    if !matches!(taker.shape, Shape::Bare | Shape::Sign) {
                        return Err(format!("The name '{}' must be followed by a word that takes it", t.lexeme));
                    }
                    self.take_named(&taker, &t.lexeme, &mut stmts, &mut stack)?;
                }
                Shape::Bare | Shape::Sign => self.bare_word(&t, &mut stmts, &mut stack)?,
                _ => unreachable!(),
            }
        }
    }

    fn drop_top(&mut self, stack: &mut Vec<Form>) -> Res<Form> {
        if let Some(n) = stack.pop() {
            return Ok(n);
        }
        // The nearest postfix program takes one more parameter.
        let Some(i) = self.layers.iter().rposition(|s| s.rpn) else {
            return if self.strict { Err("Stack underflow".to_string()) } else { Ok(constant(Value::Nil)) };
        };
        let scope = &mut self.layers[i];
        if let Some(fork) = self.forks.last_mut().filter(|f| f.layer == i) {
            if fork.from + fork.taken < scope.formals.len() {
                let name = scope.formals[fork.from + fork.taken].clone();
                fork.taken += 1;
                return Ok(self.read(&name));
            }
            fork.taken += 1;
        }
        let name = format!("p{}", scope.formals.len() + 1);
        scope.formals.push(name.clone());
        scope.idents.push(name.clone());
        scope.formal_slots.push(scope.idents.len() - 1);
        Ok(self.read(&name))
    }

    /// Whether a bare word names a binding of the routine being read, a
    /// formal or a name assigned in it so far: then it is a value, not a
    /// call, whatever routine the file binds under that name elsewhere.
    fn shadowed(&self, w: &str) -> bool {
        match self.layers.iter().rposition(|l| l.rpn) {
            Some(i) => self.layers[i].idents.iter().any(|n| n == w),
            None => false,
        }
    }

    /// A postfix branch opens: from here the arms share what the routine takes.
    fn fork(&mut self) {
        if let Some(i) = self.layers.iter().rposition(|s| s.rpn) {
            self.forks.push(Fork { layer: i, from: self.layers[i].formals.len(), taken: 0 });
        }
    }

    /// The second arm starts over on the values the first arm took.
    fn refork(&mut self) {
        if let Some(fork) = self.forks.last_mut() {
            fork.taken = 0;
        }
    }

    /// The branch closes; a branch around it has consumed all that was added.
    fn join(&mut self) {
        if let Some(done) = self.forks.pop() {
            if let Some(outer) = self.forks.last_mut().filter(|f| f.layer == done.layer) {
                outer.taken = self.layers[done.layer].formals.len() - outer.from;
            }
        }
    }

    fn flush(&mut self, stmts: &mut Vec<Form>, stack: &mut Vec<Form>) {
        for node in stack.iter_mut() {
            if !inert(node) {
                let slot = self.gensym("value");
                let value = std::mem::replace(node, Form::Read(slot.clone()));
                stmts.push(Form::Write(slot, Box::new(value)));
            }
        }
    }

    fn need_intro(&mut self) -> Res<()> {
        if self.on_any("block.intro") {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}', got '{}'", self.table.single("block.intro").unwrap_or("repeat"), self.look().lexeme))
        }
    }

    /// The body a control word governs, in the block style: indented
    /// lines, a bracketed run, or words up to a closer, which is taken.
    fn ruled(&mut self) -> Res<(Vec<Form>, Option<Form>)> {
        match self.table.blocks {
            Blocks::Indented => {
                self.skip_line_ends();
                if self.look().shape != Shape::Open {
                    return Err(format!("Expected an indented block, got '{}'", self.look().lexeme));
                }
                self.advance();
                let body = self.rpn_block(&[], Mode::Body)?;
                if self.look().shape != Shape::Close {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.advance();
                Ok(body)
            }
            Blocks::Bracketed => {
                let opens = self.table.strings("block.open");
                let k = opens.iter().position(|o| self.lexeme_of(o)).ok_or_else(|| format!("Expected '{}' to open a block, got '{}'", opens[0], self.look().lexeme))?;
                self.advance();
                let close = self.table.strings("block.close")[k].clone();
                let body = self.rpn_block(std::slice::from_ref(&close), Mode::Body)?;
                self.need_lexeme(&close)?;
                Ok(body)
            }
            Blocks::Worded => {
                let closers = self.table.strings("block.close").to_vec();
                let body = self.rpn_block(&closers, Mode::Body)?;
                self.need_closer()?;
                Ok(body)
            }
        }
    }

    /// A loop's condition, read again each pass: the words up to where the
    /// body begins by style, the line end, the opener, or the intro word.
    fn rpn_test(&mut self) -> Res<(Vec<Form>, Option<Form>)> {
        match self.table.blocks {
            Blocks::Indented => self.rpn_block(&[], Mode::Single),
            Blocks::Bracketed => {
                let opens = self.table.strings("block.open").to_vec();
                self.rpn_block(&opens, Mode::Body)
            }
            Blocks::Worded => {
                let intros = self.table.strings("block.intro").to_vec();
                let test = self.rpn_block(&intros, Mode::Body)?;
                self.need_intro()?;
                Ok(test)
            }
        }
    }

    /// Step onto an else word if one follows past line ends.
    fn grab_else(&mut self) -> bool {
        let mut ahead = 0;
        loop {
            let t = &self.tokens[(self.pos + ahead).min(self.tokens.len() - 1)];
            let separator = t.shape == Shape::LineEnd || (t.shape == Shape::Sign && self.table.separates(&t.lexeme));
            if !separator {
                if t.shape == Shape::Bare && self.table.spells("stmt.else", &t.lexeme) {
                    self.pos += ahead + 1;
                    return true;
                }
                return false;
            }
            ahead += 1;
        }
    }

    /// A body: its statements and what it leaves, at most one value.
    fn rpn_block(&mut self, stops: &[String], run: Mode) -> Res<(Vec<Form>, Option<Form>)> {
        let (mut stmts, mut rest) = self.rpn_body(stops, run)?;
        let left = match rest.len() {
            0 => None,
            1 => rest.pop(),
            n if self.strict => return Err(format!("A body may leave one value on the stack, this one leaves {}", n)),
            _ => {
                let last = rest.pop();
                stmts.extend(rest.into_iter().filter(|n| !inert(n)));
                last
            }
        };
        let returns = matches!(stmts.last(), Some(Form::Apply(Callee::Prim(Prim::Yield | Prim::Leave | Prim::Resume, _), _)));
        if returns && left.is_some() {
            stmts.push(left.unwrap());
            return Ok((stmts, None));
        }
        Ok((stmts, left))
    }

    fn cycle_body(&mut self, stmts: Vec<Form>, left: Option<Form>) -> Res<Form> {
        match left {
            None => Ok(sequence(stmts)),
            Some(_) if self.strict => Err("A loop body may not leave a value on the stack".to_string()),
            Some(n) if inert(&n) => Ok(sequence(stmts)),
            Some(n) => {
                let mut stmts = stmts;
                stmts.push(n);
                Ok(sequence(stmts))
            }
        }
    }

    fn take_named(&mut self, taker: &Token, name: &str, stmts: &mut Vec<Form>, stack: &mut Vec<Form>) -> Res<()> {
        let table = self.table;
        let word = taker.lexeme.as_str();
        if table.spells("stmt.assign", word) || table.spells("stmt.let", word) {
            let value = self.drop_top(stack)?;
            self.flush(stmts, stack);
            if let Form::Const(Value::Routine(p)) = &value {
                let arity = Signature { arity: p.formals.len(), yields: yields_value(&p.body) };
                self.seen.insert(name.to_string(), arity);
                self.presumed.insert(name.to_string(), arity);
            } else if !self.layers.iter().any(|l| l.rpn) {
                // Rebound at the top level to something else: from here on
                // the name is a value, whatever routine it named before.
                self.presumed.remove(name);
            }
            let node = self.write(name, value);
            stmts.push(node);
            return Ok(());
        }
        if table.spells("stmt.for", word) {
            let end = self.drop_top(stack)?;
            let start = self.drop_top(stack)?;
            self.flush(stmts, stack);
            self.address_to_write(name);
            let node = self.count_loop(name, start, end, |r| {
                let (s, left) = r.ruled()?;
                r.cycle_body(s, left)
            })?;
            stmts.push(node);
            return Ok(());
        }
        match table.prims.get(word).copied() {
            Some(Prim::Append) => {
                let value = self.drop_top(stack)?;
                self.flush(stmts, stack);
                let target = self.read(name);
                stmts.push(Form::Apply(Callee::Prim(Prim::Append, Rc::from(word)), vec![target, value]));
                Ok(())
            }
            Some(Prim::Replace) => {
                let value = self.drop_top(stack)?;
                let index = self.drop_top(stack)?;
                self.flush(stmts, stack);
                let target = self.read(name);
                stmts.push(Form::Apply(Callee::Prim(Prim::Replace, Rc::from(word)), vec![target, index, value]));
                Ok(())
            }
            _ => Err(format!("'{}' does not take a name, but '{}' was given", word, name)),
        }
    }

    fn bare_word(&mut self, t: &Token, stmts: &mut Vec<Form>, stack: &mut Vec<Form>) -> Res<()> {
        let table = self.table;
        let w = t.lexeme.as_str();
        let closers = table.strings("block.close").to_vec();
        for (key, v) in [("literal.true", Value::Flag(true)), ("literal.false", Value::Flag(false)), ("literal.null", Value::Nil)] {
            if table.spells(key, w) {
                stack.push(constant(v));
                return Ok(());
            }
        }
        if table.spells("stmt.if", w) {
            let test = self.drop_top(stack)?;
            self.flush(stmts, stack);
            // In the keyword style one closer ends both arms; in the
            // others each arm is a governed block.
            let keyword = table.blocks == Blocks::Worded;
            let mut stops = closers.clone();
            stops.extend(table.strings("stmt.else").iter().cloned());
            // Each arm is read inside its own program; what it leaves is its value.
            let mut then_left = false;
            let mut then_returns = false;
            self.fork();
            let then = self.limb(Traps::Naught, |r| {
                let (mut s, left) = if keyword { r.rpn_block(&stops, Mode::Body)? } else { r.ruled()? };
                then_returns = matches!(s.last(), Some(Form::Apply(Callee::Prim(Prim::Yield | Prim::Leave | Prim::Resume, _), _)));
                then_left = left.is_some();
                s.extend(left);
                Ok(sequence(s))
            })?;
            let mut else_left = false;
            let mut else_returns = false;
            let has_else = if keyword { self.on_any("stmt.else") && { self.advance(); true } } else { self.grab_else() };
            self.refork();
            let otherwise = if has_else {
                self.limb(Traps::Naught, |r| {
                    let (mut s, left) = if keyword { r.rpn_block(&closers, Mode::Body)? } else { r.ruled()? };
                    else_returns = matches!(s.last(), Some(Form::Apply(Callee::Prim(Prim::Yield | Prim::Leave | Prim::Resume, _), _)));
                    else_left = left.is_some();
                    s.extend(left);
                    Ok(sequence(s))
                })?
            } else {
                self.limb(Traps::Naught, |_| Ok(constant(Value::Nil)))?
            };
            self.join();
            if keyword {
                self.need_closer()?;
            }
            let as_value = match (then_left, else_left) {
                (false, false) => false,
                (true, true) => true,
                (true, false) if else_returns => true,
                (false, true) if then_returns => true,
                _ if self.strict => return Err("The branches of an if must leave the same number of values".to_string()),
                _ => false,
            };
            let node = self.choose(test, then, otherwise);
            if as_value {
                stack.push(node);
            } else {
                stmts.push(node);
            }
            return Ok(());
        }
        if table.spells("stmt.while", w) {
            self.flush(stmts, stack);
            let node = self.cycle(
                |r| {
                    let (s, left) = r.rpn_test()?;
                    let mut items = s;
                    items.push(left.ok_or_else(|| "A while condition must leave one value".to_string())?);
                    Ok(sequence(items))
                },
                |r| {
                    let (s, left) = r.ruled()?;
                    r.cycle_body(s, left)
                },
                None::<fn(&mut Self) -> Res<Form>>,
            )?;
            stmts.push(node);
            return Ok(());
        }
        if table.spells("stmt.until", w) {
            // Written head first as in Lumen, the condition tested after the body.
            self.flush(stmts, stack);
            let node = self.until_loop(
                |r| {
                    let (s, left) = r.ruled()?;
                    r.cycle_body(s, left)
                },
                |r| {
                    let (s, left) = r.rpn_test()?;
                    let mut items = s;
                    items.push(left.ok_or_else(|| "An until condition must leave one value".to_string())?);
                    Ok(sequence(items))
                },
            )?;
            stmts.push(node);
            return Ok(());
        }
        if table.spells("stmt.return", w) {
            let value: Vec<Form> = stack.pop().into_iter().collect();
            self.flush(stmts, stack);
            stmts.push(prim_call(Prim::Yield, value));
            return Ok(());
        }
        if table.spells("stmt.break", w) {
            self.flush(stmts, stack);
            stmts.push(prim_call(Prim::Leave, Vec::new()));
            return Ok(());
        }
        if table.spells("stmt.continue", w) {
            self.flush(stmts, stack);
            stmts.push(prim_call(Prim::Resume, Vec::new()));
            return Ok(());
        }
        if table.spells("stmt.for", w) {
            return Err(format!("'{}' needs a quoted name before it", w));
        }
        if let Some(k) = table.strings("stack.program.open").iter().position(|o| o == w) {
            let close = table.strings("stack.program.close")[k].clone();
            self.gensyms += 1;
            let name = format!("<program{}>", self.gensyms);
            self.layers.push(Layer { async_walk_seen: false, permits_async: false, gathering_kind: None, expression_targets: Vec::new(), comprehension: false, borrowed: Vec::new(), class_borrowed: Vec::new(), reaching: Vec::new(), holds: Holds::Every, idents: Vec::new(), formals: Vec::new(), formal_slots: Vec::new(), rpn: true, aliases: Vec::new(), encountered: Vec::new() });
            let (mut s, left) = self.rpn_block(std::slice::from_ref(&close), Mode::Quoted)?;
            self.need_lexeme(&close)?;
            if let Some(v) = left {
                s.push(prim_call(Prim::Yield, vec![v]));
            }
            let scope = self.layers.pop().unwrap();
            let mut params = scope.formals;
            let mut param_slots = scope.formal_slots;
            params.reverse();
            param_slots.reverse();
            let program = Routine { class_namespace: None, annotator: None, literals: Vec::new(), referenced: Vec::new(), locals: Vec::new(), flags: 0, lineless: false, qualification: String::new(), doc: None, generator: false, local_defaults: Vec::new(), gather_from: None, ident: name, least: 0, formals: params, formal_kinds: Vec::new(), taking: None, formal_slots: param_slots, idents: scope.idents, reaching: scope.reaching, frameless: false, written_in: self.written_in.clone(), within: None, declared_on: 0, type_params: Vec::new(), globe: self.globe.clone(), born: self.born.clone(), framed_in: self.framed_in.clone(), traps: Traps::Yields, carried: Vec::new(), body: sequence(s) };
            stack.push(constant(Value::Routine(Rc::new(program))));
            return Ok(());
        }
        if table.single("syntax.array.open") == Some(w) {
            let close = table.single("syntax.array.close").unwrap().to_string();
            let (inner, items) = self.rpn_body(std::slice::from_ref(&close), Mode::Body)?;
            self.need_lexeme(&close)?;
            if !inner.is_empty() {
                self.flush(stmts, stack);
                stmts.extend(inner);
            }
            stack.push(prim_call(Prim::MakeArray, items));
            return Ok(());
        }
        if table.spells("stack.dup", w) {
            let a = self.drop_top(stack)?;
            stack.push(a);
            self.flush(stmts, stack);
            let a = stack.pop().unwrap();
            let b = dup_pure(&a);
            stack.push(a);
            stack.push(b);
            return Ok(());
        }
        if table.spells("stack.drop", w) {
            let a = self.drop_top(stack)?;
            if !inert(&a) {
                self.flush(stmts, stack);
                stmts.push(a);
            }
            return Ok(());
        }
        if table.spells("stack.swap", w) || table.spells("stack.over", w) || table.spells("stack.rot", w) {
            let n = if table.spells("stack.rot", w) { 3 } else { 2 };
            let mut taken = Vec::new();
            for _ in 0..n {
                taken.push(self.drop_top(stack)?);
            }
            taken.reverse();
            stack.extend(taken);
            self.flush(stmts, stack);
            let len = stack.len();
            if table.spells("stack.swap", w) {
                stack.swap(len - 1, len - 2);
            } else if table.spells("stack.over", w) {
                let a = dup_pure(&stack[len - 2]);
                stack.push(a);
            } else {
                let a = stack.remove(len - 3);
                stack.push(a);
            }
            return Ok(());
        }
        if table.spells("stack.eval", w) {
            let program = self.drop_top(stack)?;
            let arity = match &program {
                Form::Const(Value::Routine(p)) => Signature { arity: p.formals.len(), yields: yields_value(&p.body) },
                _ if self.strict => return Err("eval needs a program written out where it is used".to_string()),
                _ => Signature { arity: 0, yields: false },
            };
            let mut args = Vec::new();
            for _ in 0..arity.arity {
                args.push(self.drop_top(stack)?);
            }
            args.reverse();
            let node = invoke(program, args);
            if arity.yields {
                stack.push(node);
            } else {
                self.flush(stmts, stack);
                stmts.push(node);
            }
            return Ok(());
        }
        if let Some(op) = table.dyadic.get(w).copied() {
            let b = self.drop_top(stack)?;
            let a = self.drop_top(stack)?;
            stack.push(prim_call(op.prim, vec![a, b]));
            return Ok(());
        }
        if let Some(op) = table.monadic.get(w).copied() {
            let a = self.drop_top(stack)?;
            stack.push(prim_call(op.prim, vec![a]));
            return Ok(());
        }
        if let Some(op) = table.prims.get(w).copied() {
            let (takes, leaves) = match op {
                Prim::Append | Prim::Replace => return Err(format!("'{}' needs a quoted name before it", w)),
                Prim::External | Prim::Span => return Err(format!("'{}' has no postfix form", w)),
                Prim::Echo | Prim::Say | Prim::Out | Prim::Tell | Prim::Dump | Prim::Raise => (1, false),
                Prim::Portray => (1, true),
                Prim::Define | Prim::Gather | Prim::Erase | Prim::Standing | Prim::Hollow => return Err(format!("'{}' has no postfix form", w)),
                Prim::CharAtIndex | Prim::Fetch | Prim::MakeReal => (2, true),
                _ => (1, true),
            };
            let mut args = Vec::new();
            for _ in 0..takes {
                args.push(self.drop_top(stack)?);
            }
            args.reverse();
            let node = Form::Apply(Callee::Prim(op, Rc::from(w)), args);
            if leaves {
                stack.push(node);
            } else {
                self.flush(stmts, stack);
                stmts.push(node);
            }
            return Ok(());
        }
        if t.shape != Shape::Bare {
            return Err(format!("Unexpected '{}'", w));
        }
        if let Some(arity) = self.presumed.get(w).copied().filter(|_| !self.shadowed(w)) {
            let mut args = Vec::new();
            for _ in 0..arity.arity {
                args.push(self.drop_top(stack)?);
            }
            args.reverse();
            let target = self.read(w);
            let node = invoke(target, args);
            if arity.yields {
                stack.push(node);
            } else {
                self.flush(stmts, stack);
                stmts.push(node);
            }
            return Ok(());
        }
        let node = self.read(w);
        stack.push(node);
        Ok(())
    }
}

fn dup_pure(node: &Form) -> Form {
    match node {
        Form::Const(v) => Form::Const(v.clone()),
        Form::Read(s) => Form::Read(s.clone()),
        Form::Apply(Callee::Prim(op, name), args) => Form::Apply(Callee::Prim(*op, name.clone()), args.iter().map(dup_pure).collect()),
        _ => unreachable!("only inert nodes are copied"),
    }
}

// ---------- numbers

/// A whole number too wide for the language to hold as one is a real
/// there, written out as a literal or worked out.
fn at_language_width(v: Value, table: &Table) -> Value {
    let figures = table.count("ext.system.real.digits").unwrap_or(math::DEFAULT_PLACES);
    if let (Some(bits), Value::Huge(n)) = (table.count("ext.system.integer.bits"), &v) {
        if n.bits() >= bits as u64 {
            return math::make_number((**n).clone(), BigInt::from(1), Some(figures));
        }
    }
    // A real written in a program is brought to the width the language
    // holds its reals in, as one worked out while it runs is, so that
    // the two are the one number and not merely near enough.
    crate::data::at_binary_width(v, table.count("ext.system.real.bits"), figures, table.lone("system.real.render") == Some("shortest"))
}

/// What a language says of a run of digits it cannot read, where it
/// gives words for that, and the kernel's own naming otherwise.
fn unreadable_numeral(text: &str, table: &Table) -> String {
    match table.single("ext.lexical.number.amiss") {
        Some(said) => said.to_string(),
        None => format!("Invalid number: {}", text),
    }
}

pub fn numeral(text: &str, table: &Table) -> Res<Value> {
    if table.flag("ext.lexical.number.separator.strict") { crate::scan::check_numeral(text, table)?; }
    let marks = table.letters("ext.lexical.number.separator");
    let powers = table.letters("ext.lexical.number.exponent");
    let bare = table.letter("lexical.number.decimal_point").map_or(false, |p| text.starts_with(p) || text.ends_with(p));
    let keep = table.flag("ext.builtin.print.real_point")
        && (bare || text.chars().any(|c| marks.contains(&c) || powers.contains(&c)));
    Ok(at_language_width(read_numeral(text, table)?, table).keeping_point(keep))
}

fn read_numeral(text: &str, table: &Table) -> Res<Value> {
    let apart = table.letters("ext.lexical.number.separator");
    let plain: String = text.chars().filter(|c| !apart.contains(c)).collect();
    if plain != text {
        let mut base = 10;
        let mut begins = 0;
        for (label, worth) in [("lexical.number.hex_prefix", 16), ("ext.lexical.number.octal_prefix", 8), ("ext.lexical.number.binary_prefix", 2)] {
            if let Some(prefix) = table.strings(label).iter().find(|p| text.starts_with(p.as_str())) {
                base = worth;
                begins = prefix.len();
                break;
            }
        }
        for (offset, mark) in text.char_indices().filter(|(_, c)| apart.contains(c)) {
            let next_digit = text[offset + mark.len_utf8()..].chars().next().map_or(false, |c| c.is_digit(base));
            let follows = if offset == begins && begins != 0 {
                table.flag("ext.lexical.number.separator.after_prefix")
            } else {
                text[..offset].chars().next_back().map_or(false, |c| c.is_digit(base))
            };
            if !follows || !next_digit {
                return Err(unreadable_numeral(text, table));
            }
        }
        return read_numeral(&plain, table);
    }
    let imaginary = table.letters("ext.lexical.number.imaginary");
    if let Some(letter) = text.chars().next_back().filter(|c| imaginary.contains(c)) {
        let coefficient: f64 = text[..text.len() - letter.len_utf8()].parse().map_err(|_| unreadable_numeral(text, table))?;
        if table.has_any("ext.builtin.complex") {
            return Ok(crate::complex::pair(table, 0.0, coefficient));
        }
        return Ok(Value::Imaginary {
            coefficient,
            unready: Rc::from(table.single("ext.lexical.number.imaginary.unready").unwrap_or("Imaginary arithmetic is not ready")),
        });
    }
    for (key, radix) in [
        ("lexical.number.hex_prefix", 16u32),
        ("ext.lexical.number.binary_prefix", 2),
        ("ext.lexical.number.octal_prefix", 8),
    ] {
        for p in table.strings(key) {
            if let Some(d) = text.strip_prefix(p.as_str()) {
                return BigInt::parse_bytes(d.as_bytes(), radix).map(Value::from_big).ok_or_else(|| unreadable_numeral(text, table));
            }
        }
    }
    // Where a language says so, a nought before more digits means those
    // digits are read in base eight.
    if table.flag("ext.lexical.number.octal_lead") && text.len() > 1 && text.starts_with('0') && text.bytes().all(|b| b.is_ascii_digit()) {
        return BigInt::parse_bytes(text[1..].as_bytes(), 8).map(Value::from_big).ok_or_else(|| unreadable_numeral(text, table));
    }
    let point = table.letter("lexical.number.decimal_point");
    if let Some(mark) = table.letter("lexical.number.base_marker").filter(|m| text.contains(*m)) {
        let (num, den) = radix_number(text, mark, point, table.letter("lexical.number.exponent_marker"))?;
        return Ok(if den == BigInt::from(1) { Value::from_big(num) } else { math::make_number(num, den, Some(digit_run(text))) });
    }
    // 1e9: the number before the letter times a power of ten, as a real.
    let powers = table.letters("ext.lexical.number.exponent");
    if let Some(at) = text.find(|c| powers.contains(&c)) {
        let (before, power) = (&text[..at], &text[at + 1..]);
        let power: i32 = power.parse().map_err(|_| unreadable_numeral(text, table))?;
        let (above, beneath) = match read_numeral(before, table)? {
            Value::Frac(e) => (e.above.clone(), e.beneath.clone()),
            Value::Small(n) => (BigInt::from(n), BigInt::from(1)),
            Value::Huge(n) => ((*n).clone(), BigInt::from(1)),
            _ => return Err(unreadable_numeral(text, table)),
        };
        let ten = BigInt::from(10).pow(power.unsigned_abs());
        let (above, beneath) = if power < 0 { (above, beneath * ten) } else { (above * ten, beneath) };
        return Ok(math::make_number(above, beneath, Some(digit_run(before))));
    }
    if let Some(p) = point {
        if let Some(dot) = text.find(p) {
            let (w, f) = (&text[..dot], &text[dot + p.len_utf8()..]);
            let scale = BigInt::from(10).pow(f.len() as u32);
            let w: BigInt = if w.is_empty() { BigInt::from(0) } else { w.parse().map_err(|_| unreadable_numeral(text, table))? };
            let f: BigInt = match (f.is_empty(), table.flag("ext.lexical.number.point.open") || table.flag("ext.lexical.number.point.bare") || table.flag("ext.lexical.number.point_edge") || table.flag("ext.lexical.number.point_open")) {
                (true, true) => BigInt::from(0),
                _ => f.parse().map_err(|_| unreadable_numeral(text, table))?,
            };
            return Ok(math::make_number(w * &scale + f, scale, Some(digit_run(text))));
        }
    }
    text.parse::<BigInt>().map(Value::from_big).map_err(|_| unreadable_numeral(text, table))
}

fn digit_run(text: &str) -> usize {
    let run: String = text.chars().filter(char::is_ascii_alphanumeric).collect();
    let zeros = run.chars().take_while(|c| *c == '0').count();
    run.len().saturating_sub(zeros).max(1).max(15)
}

fn radix_number(text: &str, mark: char, point: Option<char>, expo: Option<char>) -> Res<(BigInt, BigInt)> {
    let cut = text.find(mark).unwrap();
    let radix: u32 = text[..cut].parse().map_err(|_| format!("Invalid radix in literal '{}': radix must be decimal integer", text))?;
    if !(2..=36).contains(&radix) {
        return Err(format!("Invalid radix {}: must be between 2 and 36", radix));
    }
    let tail = &text[cut + mark.len_utf8()..];
    if tail.is_empty() {
        return Err(format!("Invalid radix-N literal '{}': missing digit_value after '{}'", text, mark));
    }
    let (body, e) = match expo.and_then(|e| tail.find(e).map(|pos| (pos, e))) {
        Some((pos, e)) => (&tail[..pos], Some(&tail[pos + e.len_utf8()..])),
        None => (tail, None),
    };
    let (int_part, frac_part) = match point.and_then(|dot| body.find(dot).map(|pos| (pos, dot))) {
        Some((pos, dot)) => (&body[..pos], Some(&body[pos + dot.len_utf8()..])),
        None => (body, None),
    };
    let digit_value = |s: &str| -> Res<BigInt> {
        let mut total = BigInt::from(0);
        for c in s.chars() {
            let dot = c.to_digit(36).ok_or_else(|| format!("Invalid radix-N literal '{}': invalid digit '{}' for radix {}", text, c, radix))?;
            if dot >= radix {
                return Err(format!("Invalid radix-N literal '{}': digit '{}' (value {}) is not valid in radix {}", text, c, dot, radix));
            }
            total = total * radix + dot;
        }
        Ok(total)
    };
    if int_part.is_empty() {
        return Err(format!("Invalid radix-N literal '{}': missing digit_value", text));
    }
    let mut above = digit_value(int_part)?;
    let mut beneath = BigInt::from(1);
    match frac_part {
        Some(f) if !f.is_empty() => {
            beneath = BigInt::from(radix).pow(f.len() as u32);
            above = above * &beneath + digit_value(f)?;
        }
        Some(_) => return Err(format!("Invalid radix-N literal '{}': missing digit_value after '.'", text)),
        None => {}
    }
    if let Some(e) = e {
        if e.is_empty() {
            return Err(format!("Invalid radix-N literal '{}': missing digit_value after exponent marker", text));
        }
        let e = digit_value(e)?.to_u32().ok_or_else(|| format!("Invalid radix-N literal '{}': exponent too large", text))?;
        above *= BigInt::from(radix).pow(e);
    }
    Ok((above, beneath))
}

/// Whether the path being written includes a span of places.
fn slice_target(place: &Form) -> bool {
    match place {
        Form::Apply(Callee::Prim(Prim::At, _), given) if given.len() == 2 => {
            matches!(&given[1], Form::Apply(Callee::Prim(Prim::SliceBounds, _), _)) || slice_target(&given[0])
        }
        _ => false,
    }
}

fn borrows_enclosing(form: &Form, names: &[&str]) -> bool {
    let uses = |address: &Address| names.contains(&address.ident.as_ref());
    match form {
        Form::Read(place) | Form::Glance(place) | Form::Share(place) => uses(place),
        Form::Write(_, value) | Form::OnLine(_, value) | Form::Located(_, value) | Form::Muted(value) | Form::Silenced(value) => borrows_enclosing(value, names),
        Form::Apply(Callee::Code(target), args) => borrows_enclosing(target, names) || args.iter().any(|arg| borrows_enclosing(arg, names)),
        Form::Apply(_, args) => args.iter().any(|arg| borrows_enclosing(arg, names)),
        Form::Const(Value::Routine(arm)) if arm.frameless => borrows_enclosing(&arm.body, names),
        Form::Cycle { test, body, step, otherwise, .. } => borrows_enclosing(test, names) || borrows_enclosing(body, names)
            || step.as_deref().map_or(false, |part| borrows_enclosing(part, names)) || otherwise.as_deref().map_or(false, |part| borrows_enclosing(part, names)),
        Form::Dyad { a, b, .. } => [a, b].iter().any(|part| match part {
            Input::Address(place) => uses(place), Input::Form(form) => borrows_enclosing(form, names), _ => false,
        }),
        Form::Assert { condition, message } => borrows_enclosing(condition, names) || borrows_enclosing(message, names),
        Form::Attempt { body, clauses, last, otherwise, .. } => borrows_enclosing(body, names)
            || clauses.iter().any(|clause| borrows_enclosing(&clause.body, names))
            || last.as_deref().map_or(false, |part| borrows_enclosing(part, names)) || otherwise.as_deref().map_or(false, |part| borrows_enclosing(part, names)),
        _ => false,
    }
}

fn inspect_form(form: &Form, locals: &[String], constants: &mut Vec<Value>, names: &mut Vec<String>, suspension: &mut bool) {
    fn literal(value: &Value, constants: &mut Vec<Value>) {
        if let Value::Routine(p) = value {
            if p.frameless { return; }
            if let Some(a) = &p.annotator { literal(&Value::Routine(a.clone()), constants); }
        }
        if constants.iter().all(|v| std::mem::discriminant(v) != std::mem::discriminant(value) || !v.equals(value)) { constants.push(value.clone()); }
    }
    fn named(name: &str, locals: &[String], names: &mut Vec<String>) {
        if !name.is_empty() && !name.starts_with('#') && !locals.iter().any(|s| s == name) && !names.iter().any(|s| s == name) { names.push(name.to_owned()); }
    }
    let mut children: Vec<&Form> = Vec::new();
    match form {
        Form::Const(Value::Routine(p)) if p.frameless => children.push(&p.body),
        Form::Const(v) => literal(v, constants),
        Form::Read(a) | Form::Take(a) | Form::Glance(a) => { if a.up > 0 && a.fallback.is_none() { named(&a.ident, locals, names); } },
        Form::Write(_, child) | Form::OnLine(_, child) | Form::Located(_, child) | Form::Muted(child) | Form::Silenced(child) | Form::Tie(_, child) => children.push(child),
        Form::Apply(callee, args) => {
            if matches!(callee, Callee::Prim(Prim::Suspend, _)) { *suspension = true; }
            match callee {
                Callee::Code(target) => children.push(target),
                Callee::Prim(op, spelling) => if !matches!(op, Prim::Seq | Prim::Choose | Prim::Both | Prim::Either | Prim::Yield | Prim::Leave | Prim::Resume) { named(spelling, locals, names); },
            }
            children.extend(args);
        }
        Form::Dyad { a, b, .. } => {
            for input in [a, b] {
                match input {
                    Input::Form(f) => children.push(f),
                    Input::Const(v) => literal(v, constants),
                    Input::Address(a) => if a.up > 0 && a.fallback.is_none() { named(&a.ident, locals, names); },
                }
            }
        }
        Form::Cycle { test, body, step, otherwise, .. } => {
            children.extend([test.as_ref(), body.as_ref()]);
            children.extend(step.as_deref());
            children.extend(otherwise.as_deref());
        }
        Form::Attempt { body, clauses, last, otherwise, .. } => {
            children.push(body);
            for clause in clauses { children.push(&clause.body); if let Some(choices) = &clause.choices { children.extend(choices); } }
            children.extend(otherwise.as_deref());
            children.extend(last.as_deref());
        }
        Form::Class { values, .. } => children.extend(values),
        Form::Assert { condition, message } => children.extend([condition.as_ref(), message.as_ref()]),
        _ => {}
    }
    for child in children { inspect_form(child, locals, constants, names, suspension); }
}
