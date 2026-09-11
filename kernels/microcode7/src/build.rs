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

#[derive(Clone, Default)]
struct ScopeWords {
    bound: Vec<String>,
    outermost: Vec<(String, String)>,
    borrowed: Vec<String>,
}

struct Layer {
    comprehension: bool,
    borrowed: Vec<String>,
    holds: Holds,
    idents: Vec<String>,
    formals: Vec<String>,
    formal_slots: Vec<usize>,
    rpn: bool,
    /// Names bound to a global (by `global`, the same name; by `static`, a hidden one).
    aliases: Vec<(String, String)>,
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
    /// The class being read and what it is built on: what `self` and
    /// `parent` mean inside a method.
    within: Option<(String, Option<String>)>,
    receiver: Option<String>,
    class_bindings: Vec<(usize, HashMap<String, Address>)>,
    /// Which parameters of each program take a name's own cell instead
    /// of a copy, read from the tokens before anything is built.
    shared_args: HashMap<String, Vec<bool>>,
    arg_names: HashMap<String, Vec<String>>,
    /// Whether the reading stopped over a thing the language calls a
    /// fault of the run rather than a program it could not read.
    stopped_fatally: bool,
    /// The line the routine now being read was written on, which a
    /// fault raised on the way into it names.
    declared_at: u32,
    /// The slots the routine being built fills from what it carried
    /// away with it, gathered while its names are read.
    carrying: Vec<usize>,
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
    pos: usize,
    layers: Vec<Layer>,
    /// Whether this text was handed over while the run was already
    /// going, as text given to the word that reads text is and as a
    /// file asked for part way through is. A whole program built this
    /// way is a piece of a run in progress, not a run of its own.
    read_in: bool,
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
    /// The file this text came out of, where it was read as the run
    /// went, so that every program built from it carries it.
    written_in: Option<Rc<str>>,
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
    source_before: Option<(usize, usize, String)>,
    unsupported_place: bool,
    iteration_binding: Option<(String, usize)>,
    place_depth: usize,
    outside_lambda: Vec<String>,
}

pub struct Built {
    pub program: Rc<Routine>,
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
    build_marking(tokens, table, seeded, assumed, strict, before, None, None, None, None, false)
}

/// The same, saying besides which row the reading had reached when it
/// stopped, for a language that tells such a stopping in its own words.
pub fn build_at(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32) -> Result<Built, (String, u32, bool)> {
    let at = std::cell::Cell::new(0u32);
    let hard = std::cell::Cell::new(false);
    build_marking(tokens, table, seeded, assumed, strict, before, None, Some((&at, &hard)), None, None, false)
        .map_err(|said| (said, at.get(), hard.get()))
}

/// The same, said besides which file the text came out of. Nothing but
/// text read in while the run was already going is built this way, so a
/// statement that means one thing in a program of its own and another
/// in a piece of a run in progress can tell the two apart.
pub fn build_from(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32, written_in: Option<Rc<str>>) -> Res<Built> {
    build_marking(tokens, table, seeded, assumed, strict, before, written_in, None, None, None, true)
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
    build_marking(tokens, table, seeded, HashMap::new(), true, before, None, None, Some((inside, knows)), within, true)
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
) -> Result<Built, (String, u32)> {
    let (at, hard) = (std::cell::Cell::new(0u32), std::cell::Cell::new(false));
    build_marking(tokens, table, seeded, HashMap::new(), true, before, None, Some((&at, &hard)), Some((inside, knows)), within, true).map_err(|said| (said, at.get()))
}

pub fn build_from_at(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32) -> Result<Built, (String, u32)> {
    let (at, hard) = (std::cell::Cell::new(0u32), std::cell::Cell::new(false));
    build_marking(tokens, table, seeded, assumed, strict, before, None, Some((&at, &hard)), None, None, true).map_err(|said| (said, at.get()))
}

/// Text handed over to be read while the run goes: as one expression
/// and nothing after it where it is to be weighed, else as statements;
/// said besides which file it came out of, and which builtin words are
/// to be read as names the program bound, in front of the builtins.
pub fn build_text(tokens: &[Token], table: &Table, seeded: &[String], before: u32, written_in: Option<Rc<str>>, value_only: bool, shadowed: &[String]) -> Res<Built> {
    let mut words = HashMap::new();
    if table.flag("ext.stmt.function.closes_over") {
        build_survey(tokens, table, seeded, HashMap::new(), true, before, written_in.clone(), None, None, None, true, &mut words, true, value_only, shadowed)?;
    }
    build_survey(tokens, table, seeded, HashMap::new(), true, before, written_in, None, None, None, true, &mut words, false, value_only, shadowed)
}

type Knows<'w> = (&'w HashMap<String, Vec<bool>>, &'w HashMap<String, Vec<String>>, &'w HashSet<String>);
type Within<'w> = (&'w [String], Knows<'w>);

fn build_marking(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32, written_in: Option<Rc<str>>, mark: Option<(&std::cell::Cell<u32>, &std::cell::Cell<bool>)>, within: Option<Within>, standing_in: Option<(String, Option<String>)>, read_in: bool) -> Res<Built> {
    let mut words = HashMap::new();
    if table.flag("ext.stmt.function.closes_over") {
        build_survey(tokens, table, seeded, assumed.clone(), strict, before, written_in.clone(), mark, within, standing_in.clone(), read_in, &mut words, true, false, &[])?;
    }
    build_survey(tokens, table, seeded, assumed, strict, before, written_in, mark, within, standing_in, read_in, &mut words, false, false, &[])
}

fn build_survey(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool, before: u32, written_in: Option<Rc<str>>, mark: Option<(&std::cell::Cell<u32>, &std::cell::Cell<bool>)>, within: Option<Within>, standing_in: Option<(String, Option<String>)>, read_in: bool, words: &mut HashMap<usize, ScopeWords>, survey: bool, value_only: bool, shadowed: &[String]) -> Res<Built> {
    let mut beginnings = seeded.to_vec();
    for word in table.strings("ext.builtin.exceptions") {
        if !beginnings.contains(word) { beginnings.push(word.clone()); }
    }
    let top = Layer { comprehension: false, borrowed: Vec::new(), holds: Holds::Every, idents: beginnings, formals: Vec::new(), formal_slots: Vec::new(), rpn: false, aliases: Vec::new() };
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
        layers.push(Layer { comprehension: false, borrowed: Vec::new(), holds: Holds::Fresh, idents: inside.to_vec(), formals: Vec::new(), formal_slots: Vec::new(), rpn: false, aliases: Vec::new() });
    }
    let outer_layers = layers.len();
    let mut r = Builder { kind_mark: None, surveyed: words.clone(), survey, class_bindings: Vec::new(), receiver: None, outside_lambda: Vec::new(), within: standing_in, bags: HashMap::new(), shared_args, arg_names, gives_back, stopped_fatally: false, declared_at: 0, carrying: Vec::new(), noted_when_read: Vec::new(), table, forks: Vec::new(), tokens, pos: 0, layers, read_in, outer_layers, spoken_for: Vec::new(), gensyms: 0, gather_names: Vec::new(), presumed: assumed, seen: HashMap::new(), strict, statics: Vec::new(), also_property: Vec::new(), before, past_library: false, named_in_program: shadowed.to_vec(), written_in, waiting: None, stepping: None, stood: None, stands: None, naming: Vec::new(), giving_cells: Vec::new(), formal_kinds: Vec::new(), taking: None,
        generator_seen: false,
        reading_yield: false, place_depth: 0,
        source_before: None,
        unsupported_place: false,
        iteration_binding: None,
        tells_place: ["ext.system.complaint.warning", "ext.system.complaint.notice", "ext.system.complaint.deprecated", "ext.system.complaint.fatal"]
            .iter()
            .any(|key| table.single(key).is_some()) || table.flag("ext.system.source.marked") };
    let body = if value_only {
        // One expression, with line ends about it and nothing else.
        r.skip_line_ends();
        let value = r.comma_value()?;
        r.skip_line_ends();
        if !r.exhausted() {
            return Err(format!("Unexpected '{}'", r.look().lexeme));
        }
        value
    } else if table.rpn {
        let (mut stmts, rest) = match r.rpn_body(&[], Mode::Body) {
            Ok(got) => got,
            Err(said) => {
                if let Some((mark, hard)) = mark {
                    mark.set(r.look().row);
                    hard.set(r.stopped_fatally);
                }
                return Err(said);
            }
        };
        if !r.exhausted() {
            if let Some((mark, hard)) = mark {
                mark.set(r.look().row);
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
            // Where the reading stops, the row it had reached is kept,
            // so a language with a word for such a stopping names it.
            let stmt = match r.stmt() {
                Ok(stmt) => stmt,
                Err(said) => {
                    if let Some((mark, hard)) = mark {
                        mark.set(r.look().row);
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
    let program = Routine { qualification: String::new(), doc: None, generator: false, local_defaults: Vec::new(), gather_from: None, ident: "<program>".into(), least: 0, formals: Vec::new(), formal_kinds: Vec::new(), taking: None, formal_slots: Vec::new(), idents: top.idents, frameless: false, written_in: r.written_in.clone(), within: None, declared_on: 0, traps: Traps::Naught, carried: Vec::new(), body };
    Ok(Built { program: Rc::new(program), globals, outer_aliases, seen: r.seen, shared_args: r.shared_args, arg_names: r.arg_names, gives_back: r.gives_back })
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
    /// `global` or `static` statement bound it.
    fn aliased(&mut self, name: &str) -> Option<Address> {
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
        for index in (1..self.layers.len()).rev() {
            let layer = &self.layers[index];
            if layer.holds == Holds::Nothing { continue; }
            if !(borrowed && index == owner) {
                if layer.aliases.iter().any(|pair| pair.0 == name) { return None; }
                if let Some(at) = layer.idents.iter().rposition(|word| word == name) {
                    return Some(Address { ident: Rc::from(name), up: distance, at, fallback: None });
                }
            }
            distance += 1;
        }
        None
    }

    fn address_to_read(&mut self, name: &str) -> Address {
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
        let private = self.gather_names.iter().rfind(|pair| pair.0 == name).map(|pair| pair.1.to_string());
        let name = private.as_deref().unwrap_or(name);
        if self.table.flag("ext.stmt.function.closes_over") && !name.starts_with('#') {
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

    /// The name of an array being written into, addressed as the write
    /// it is: a name first met on the left of a write belongs to the
    /// scope it is written in, and every later reading of it must find
    /// the same cell.
    fn read_to_write(&mut self, name: &str) -> Form {
        let slot = self.address_to_write(name);
        Form::Read(slot)
    }

    fn read(&mut self, name: &str) -> Form {
        if let Some((depth, names)) = self.class_bindings.last() {
            if *depth == self.layers.len() {
                if let Some(slot) = names.get(name) { return Form::Read(slot.clone()); }
            }
        }
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
        Form::Read(self.address_to_read(name))
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

    /// A program value: its body reduced in a scope of its own.
    fn routine(&mut self, name: &str, holds: Holds, catches: Traps, params: Vec<String>, least: usize, body: impl FnOnce(&mut Self) -> Res<Form>) -> Res<Form> {
        let began = self.pos;
        let mut start = self.pos;
        while self.tokens.get(start).map_or(false, |t| matches!(t.shape, Shape::Open | Shape::LineEnd) || self.table.spells("block.intro", &t.lexeme)) { start += 1; }
        let doc = self.tokens.get(start).filter(|t| t.shape == Shape::Quote).map(|t| t.lexeme.to_owned());
        let mut path=self.within.as_ref().map(|(c,_)|format!("{c}.")).unwrap_or_default();
        let local=self.table.single("ext.stmt.class.detail.locals").unwrap_or("");
        for outer in &self.naming {path.push_str(outer);path.push('.');path.push_str(local);path.push('.');}
        path.push_str(name);
        let qualification=path;
        let taking = self.taking.take();
        let declared_on = self.declared_at;
        // What the routine around this one carries is set aside while
        // this one is built, so that each keeps only its own.
        let around = std::mem::take(&mut self.carrying);
        self.naming.push(name.to_string());
        // The classes the parameters were written to take, gathered as
        // they were read. A method is handed the thing it is for before
        // them, so the list is brought level with the names.
        let mut formal_kinds = std::mem::take(&mut self.formal_kinds);
        while formal_kinds.len() < params.len() {
            formal_kinds.insert(0, None);
        }
        formal_kinds.truncate(params.len());
        let param_slots = (0..params.len()).collect();
        self.layers.push(Layer { comprehension: name == "<gathering>", borrowed: Vec::new(), holds, idents: params.clone(), formals: Vec::new(), formal_slots: param_slots, rpn: false, aliases: Vec::new() });
        if self.table.flag("ext.stmt.function.closes_over") && !self.survey && holds == Holds::Every {
            if let Some(known) = self.surveyed.get(&began) {
                if known.borrowed.iter().any(|word| params.contains(word)) {
                    return Err(self.table.single("ext.stmt.function.parameters.amiss").unwrap_or_default().into());
                }
                let scope = self.layers.last_mut().unwrap();
                for word in &known.bound {
                    if !scope.idents.contains(word) { scope.idents.push(word.clone()); }
                }
                scope.aliases.clone_from(&known.outermost);
                scope.borrowed.clone_from(&known.borrowed);
            }
        }
        let earlier_gathering = self.gather_names.clone();
        if holds == Holds::Every && self.table.flag("ext.stmt.function.closes_over") {
            let own = &self.layers.last().unwrap().idents;
            self.gather_names.retain(|pair| !own.contains(&pair.0));
        }
        let enclosing_yield = self.generator_seen;
        if holds == Holds::Every { self.generator_seen = false; }
        let mut body = body(self)?;
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
            self.surveyed.insert(began, ScopeWords { bound, outermost: scope.aliases.clone(), borrowed: scope.borrowed.clone() });
        }
        if generator && !self.table.flag("ext.stmt.function.closes_over") {
            let names = self.layers.iter().skip(1).filter(|scope| scope.holds == Holds::Every)
                .flat_map(|scope| scope.idents.iter()).filter(|name| !scope.idents.contains(name))
                .map(String::as_str).collect::<Vec<_>>();
            if borrows_enclosing(&body, &names) { body = self.scope_unrun("ext.stmt.yield.unsupported"); }
        }
        self.naming.pop();
        let carried = std::mem::replace(&mut self.carrying, around);
        Ok(constant(Value::Routine(Rc::new(Routine { qualification, doc, generator, local_defaults: Vec::new(), gather_from: None, ident: name.to_string(), least, formals: params, taking, formal_kinds, formal_slots: scope.formal_slots, idents: scope.idents, frameless: holds == Holds::Nothing, written_in: self.written_in.clone(), within: self.within.as_ref().map(|(named, _)| Rc::from(named.as_str())), declared_on, traps: catches, carried, body }))))
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
            let program = Routine { qualification: String::new(), doc: None, generator: false, local_defaults: Vec::new(), gather_from: None, ident: name.to_string(), least: 0, formals: Vec::new(), formal_kinds: Vec::new(), taking: None, formal_slots: Vec::new(), idents: Vec::new(), frameless: true, written_in: self.written_in.clone(), within: None, declared_on: 0, traps: Traps::Naught, carried: Vec::new(), body };
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
        let body = body(self)?;
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

    fn stmt(&mut self) -> Res<Form> {
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
    fn comma_value(&mut self) -> Res<Form> {
        let (first, spreads) = self.comma_part()?;
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
            let (part, spread) = self.comma_part()?;
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
            let (part, expanded) = self.comma_part()?;
            let segment = if expanded { part } else { prim_call(Prim::MakeArray, vec![part]) };
            value = prim_call(Prim::TupleJoined, vec![value, segment]);
            if !self.on_any("ext.op.tuple") { break; }
        }
        Ok(if self.table.has_any("ext.builtin.tuple") { prim_call(Prim::Tupling, vec![value]) } else { value })
    }

    fn plain_or_kind(&mut self) -> Res<Form> {
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
            if self.key("ext.stmt.async") {
                let word = self.advance().lexeme;
                if !self.key("stmt.function") && !self.key("stmt.for") && !self.key("ext.stmt.with") {
                    return Err(format!("Expected a function, for loop or with block after '{}', got '{}'", word, self.look().lexeme));
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
                    return Err(self.table.single("ext.stmt.nonlocal.module").unwrap_or_default().into());
                }
                let in_class = self.class_bindings.last().map_or(false, |(level, _)| *level == self.layers.len());
                self.advance();
                loop {
                    let word = self.need_word("after the nonlocal keyword")?;
                    if !in_class {
                        if let Some(layer) = self.layers.iter_mut().rev().find(|l| l.holds == Holds::Every) { layer.borrowed.push(word.clone()); }
                    }
                    if !in_class && !self.survey && self.table.flag("ext.stmt.function.closes_over") && self.lexical_address(&word, true).is_none() {
                        let pieces = self.table.strings("ext.stmt.nonlocal.amiss");
                        return Err(format!("{}{}{}", pieces.first().map_or("", String::as_str), word, pieces.get(1).map_or("", String::as_str)));
                    }
                    if !self.on_any("syntax.call.separator") { break; }
                    self.advance();
                }
                if in_class { return Ok(self.class_not_ready()); }
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
                self.advance();
                self.need_word("as the type alias")?;
                self.type_names()?;
                self.need_assign("after the type alias")?;
                self.put_by_annotation(&[])?;
                return Ok(constant(Value::Nil));
            }
            if self.key("stmt.return") {
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
                self.advance();
                let levels = self.loop_levels()?;
                return Ok(prim_call(Prim::Leave, levels));
            }
            if self.key("stmt.continue") {
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
                if self.table.single("ext.stmt.throw.from").is_some()
                    && (matches!(self.look().shape, Shape::LineEnd | Shape::Close | Shape::Finish) || self.on_any("stmt.terminator"))
                {
                    return Ok(Form::Again);
                }
                let mut values = vec![self.expr(0)?];
                if self.key("ext.stmt.throw.from") {
                    self.advance();
                    let cause = self.expr(0)?;
                    if self.table.has_any("ext.builtin.exceptions") { values.push(cause); }
                }
                return Ok(prim_call(Prim::Hurl, values));
            }
            if self.key("ext.stmt.assert") {
                self.advance();
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
            let said = self.need_word("among imported names")?;
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

    /// Paths remain whole, to be fetched when this statement is reached.
    fn import_bindings(&mut self) -> Res<Form> {
        let taking_names = self.key("ext.stmt.import.from");
        self.advance();
        let mut path = String::new();
        if taking_names {
            let start = self.pos;
            while self.on_any("op.pipe") || self.on_any("ext.op.index.slice.ellipsis") {
                path.push_str(&self.look().lexeme);
                self.advance();
            }
            if self.pos == start || !self.key("ext.stmt.import") {
                path.push_str(&self.module_path(true)?);
            }
            if self.key("ext.stmt.import") {
                self.advance();
            } else {
                return Err(format!("Expected '{}' following the module path, got '{}'", self.table.single("ext.stmt.import").unwrap_or_default(), self.look().lexeme));
            }
        }
        let enclosed = taking_names && self.on_any("syntax.group.open");
        if enclosed {
            self.advance();
        }
        let mut writes = Vec::new();
        if taking_names && !enclosed && self.on_any("op.mul") {
            self.advance();
            if self.table.flag("ext.stmt.import.value") {
                writes.push(prim_call(Prim::SpreadModule, vec![prim_call(Prim::BringModule, vec![constant(Value::text(&path)), constant(Value::Nil), constant(Value::Flag(false))])]));
            }
        } else {
            loop {
                let original = self.module_path(!taking_names)?;
                let alias = self.key("ext.stmt.import.as");
                let local = match alias {
                    false => original.split(self.table.single("op.pipe").unwrap_or(".")).next().unwrap_or(&original).to_string(),
                    true => {
                        self.advance();
                        self.module_path(false)?
                    }
                };
                let worth = if self.table.flag("ext.stmt.import.value") {
                    prim_call(Prim::BringModule, vec![constant(Value::text(if taking_names { &path } else { &original })), constant(if taking_names { Value::text(&original) } else { Value::Nil }), constant(Value::Flag(!taking_names && !alias))])
                } else { constant(Value::Nil) };
                writes.push(self.write(&local, worth));
                if !self.on_any("syntax.call.separator") {
                    break;
                }
                self.advance();
                if enclosed && self.on_any("syntax.group.close") {
                    break;
                }
            }
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

    /// The writes before the definition gather its decorators. Those
    /// after it rebind the name, taking the gathered values backwards.
    fn decorate(&mut self) -> Res<Form> {
        let mut forms = Vec::new();
        let mut decorators = Vec::new();
        loop {
            let mark = self.advance().lexeme;
            let value = self.expr(0)?;
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
        if self.key("ext.stmt.async") { self.advance(); }
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
        self.advance();
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
            let mut value = self.expr(0)?;
            if table.strings("ext.stmt.class.special").get(33).is_some() {
                let manager = self.gensym("manager");
                let manager_read = Form::Read(manager.clone());
                steps.push(Form::Write(manager.clone(), Box::new(value)));
                value = prim_call(Prim::StartContext, vec![manager_read]);
                let entered = self.gensym("entered");
                steps.push(Form::Write(entered.clone(), Box::new(value)));
                value = Form::Read(entered);
                contexts.push((steps.len(), manager));
            }
            if self.key("ext.stmt.with.as") {
                self.advance();
                let place = self.gensym("with");
                let name = place.ident.to_string();
                steps.push(Form::Write(place, Box::new(value)));
                steps.extend(self.with_target(&name)?);
            } else { steps.push(value); }
            if !self.on_any("syntax.call.separator") { break; }
            self.advance();
            if enclosed && self.sign(close) { break; }
        }
        if enclosed { self.need_sign(close, "after the with items")?; }
        steps.push(self.body()?);
        for (from, manager) in contexts.into_iter().rev() {
            let enclosed = sequence(steps.split_off(from));
            steps.push(Form::Attempt { context: Some(manager), body: Box::new(enclosed), clauses: Vec::new(), last: None, otherwise: None });
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
        let target = self.deletion_place()?;
        if self.unsupported_place {
            return Ok(vec![prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.stmt.binding.unrun").unwrap_or_default()))])]);
        }
        let previous = self.waiting.replace(source.to_string());
        let written = self.write_into(target, false, None, token);
        self.waiting = previous;
        Ok(vec![written?])
    }

    fn deletion_place(&mut self) -> Res<Form> {
        let name = self.need_word("as a binding target")?;
        let read = self.read(&name);
        let mut place = self.called_on_value(read)?;
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
                let owner = self.layers.iter_mut().rev().find(|s| s.holds == Holds::Every).expect("the top layer");
                owner.aliases.push((name.clone(), name));
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
    fn attempt_stmt(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
        let bare_clauses = table.single("ext.stmt.catch.as").is_some();
        let body = if bare_clauses { self.watched_body()? } else { self.body()? };
        let open = table.single("syntax.group.open").ok_or_else(|| "A catch needs syntax.group".to_string())?.to_string();
        let close = table.single("syntax.group.close").unwrap().to_string();
        let mut clauses = Vec::new();
        // A clause may stand on a line of its own, after what it follows.
        self.skip_line_ends();
        while self.key("ext.stmt.catch") {
            self.advance();
            let mut classes = Vec::new();
            let mut choices = None;
            let grouped = bare_clauses && self.on_any("ext.stmt.catch.group");
            if grouped { self.advance(); }
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
                            Form::Read(place) => Form::Glance(place),
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
                held = if self.key("ext.stmt.catch.as") {
                    self.advance();
                    let binding = self.need_word("after the caught value's binding word")?;
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
            let body = if bare_clauses { self.watched_body()? } else { self.body()? };
            clauses.push(Clause { classes, choices, grouped, takes_all, held, body });
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
            let limb = self.watched_body()?;
            self.skip_line_ends();
            Some(Box::new(limb))
        } else { None };
        let last = match self.key("ext.stmt.finally") {
            true => {
                self.advance();
                Some(Box::new(if bare_clauses { self.watched_body()? } else { self.body()? }))
            }
            false => None,
        };
        if clauses.is_empty() && (last.is_none() || otherwise.is_some()) {
            return Err("A try needs a catch or a last part".to_string());
        }
        Ok(Form::Attempt { context: None, body: Box::new(body), clauses, last, otherwise })
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
            if self.key("ext.stmt.async") { self.advance(); }
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

    fn class_with_receiver(&mut self) -> Res<(Form, bool)> {
        self.advance();
        let named = self.need_word("as the class name")?;
        if self.on_any("ext.stmt.type_params.open") { self.class_type_parameters()?; }
        let table = self.table;
        let mut setup = Vec::new();
        let mut parent = None;
        let mut other_parents = Vec::new();
        let mut handed_words: Vec<(String, Form)> = Vec::new();
        let mut cannot = self.layers.iter().filter(|s| s.holds == Holds::Every).count() > 1;
        if table.single("ext.stmt.class.bases.open").map_or(false, |o| self.sign(o)) {
            self.advance();
            let end = table.single("ext.stmt.class.bases.close").ok_or("The bases need a closing mark")?;
            let mut first = true;
            while !self.sign(end) {
                let expanded = table.spells("op.mul", &self.look().lexeme) || table.spells("op.pow", &self.look().lexeme);
                if expanded { self.advance(); cannot = true; }
                let keyword = self.look().shape == Shape::Bare && table.spells("stmt.assign", &self.glance(1).lexeme);
                // A keyword in the header goes, under a name no program
                // can spell, to the forebears' subclass hook; a table
                // without such a hook cannot run the form, nor can any
                // table run the keyword that names a metaclass.
                let mut handed = None;
                if keyword {
                    let word = self.advance().lexeme;
                    self.advance();
                    if table.has_any("ext.stmt.class.detail.subclass") && !table.spells("ext.stmt.class.metaclass", &word) { handed = Some(word); } else { cannot = true; }
                }
                let value = self.expr(0)?;
                if let Some(word) = handed {
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
        let previous = self.within.replace((named.clone(), parent.as_ref().map(|s| s.ident.to_string())));
        let full_name=previous.as_ref().map_or(named.clone(),|(n,_)|format!("{n}.{named}"));
        if table.has_any("ext.stmt.class.detail.root"){self.within=Some((full_name.clone(),parent.as_ref().map(|a|a.ident.to_string())));}
        self.need_intro()?;
        let on_one_line = !self.on_stmt_end() && self.look().shape != Shape::Open;
        if !on_one_line {
            self.skip_line_ends();
            if self.look().shape != Shape::Open { return Err("Expected an indented class body".to_string()); }
            self.advance();
            self.skip_line_ends();
        }
        let mut methods = Vec::new();
        self.class_bindings.push((self.layers.len(), HashMap::new()));
        let mut attributes = Vec::new();
        let mut annotated_names: Vec<(Value, Value)> = Vec::new();
        let mut values = Vec::new();
        if let Some(slot) = &parent { values.push(Form::Read(slot.clone())); }
        values.extend(other_parents.iter().cloned().map(Form::Read));
        if let Some(word)=table.single("ext.stmt.class.detail.qualified") {attributes.push(word.to_string());values.push(constant(Value::text(&full_name)));}
        let before_body = setup.len();
        while !matches!(self.look().shape, Shape::Finish | Shape::Close) {
            if on_one_line && self.on_stmt_end() { break; }
            if !table.has_any("ext.stmt.class.detail.root") && self.on_any("ext.stmt.decorator") {
                let (word, address) = self.member_adornments(&mut setup)?;
                cannot |= table.single("ext.stmt.class.constructor") == Some(word.as_str());
                methods.retain(|(n, _)| n != &word);
                if let Some(index) = attributes.iter().position(|n| n == &word) {
                    attributes.remove(index);
                    values.remove(index + usize::from(parent.is_some()));
                }
                attributes.push(word);
                values.push(Form::Read(address));
                self.skip_line_ends();
                continue;
            }
            let mut wrappers=Vec::new();
            while table.has_any("ext.stmt.class.detail.root") && self.on_any("ext.stmt.decorator") {
                self.advance();let expression=self.expr(0)?;let address=self.gensym("member_wrapper");
                setup.push(Form::Write(address.clone(),Box::new(expression)));wrappers.push(address);
                self.skip_line_ends();
            }
            if !wrappers.is_empty() && !self.key("stmt.function") && !self.key("ext.stmt.class") {cannot=true;}
            if self.key("stmt.function") {
                self.advance();
                let method_name = self.need_word("as the method name")?;
                let body = self.method(&method_name)?;
                methods.retain(|(old, _)| old != &method_name);
                if let Some(i) = attributes.iter().position(|old| old == &method_name) {
                    attributes.remove(i);
                    values.remove(i + usize::from(parent.is_some()) + other_parents.len());
                }
                let slot = self.gensym("method_body");
                let decorated=!wrappers.is_empty();
                let mut expression=constant(Value::Routine(body.clone()));
                while let Some(address)=wrappers.pop(){expression=Form::Apply(Callee::Code(Box::new(Form::Read(address))),vec![expression]);}
                setup.push(Form::Write(slot.clone(),Box::new(expression)));
                if decorated {attributes.push(method_name.clone());values.push(Form::Read(slot.clone()));}
                self.class_bindings.last_mut().expect("the class namespace").1.insert(method_name.clone(), slot);
                if !decorated {methods.push((method_name, body));}
            } else if self.key("stmt.pass") || self.look().shape == Shape::Quote {
                self.advance();
            } else {
                let member = self.look().lexeme.clone();
                let value = if self.key("ext.stmt.class") {
                    let member = self.glance(1).lexeme.clone();
                    setup.push(self.class_with_receiver()?.0);
                    attributes.push(member.clone());
                    let mut nested = self.read(&member);
                    while let Some(address) = wrappers.pop() { nested = Form::Apply(Callee::Code(Box::new(Form::Read(address))), vec![nested]); }
                    Some(nested)
                } else if self.look().shape == Shape::Bare && table.spells("stmt.assign", &self.glance(1).lexeme) {
                    self.pos += 2;
                    attributes.push(member);
                    // Commas after the value gather a tuple for the member,
                    // where the language has them.
                    let gathered = table.has_any("ext.op.tuple");
                    Some(if gathered { self.comma_value()? } else { self.expr(0)? })
                } else if self.look().shape == Shape::Bare && !table.keywords.contains(&self.look().lexeme)
                    && table.spells("ext.stmt.annotation", &self.glance(1).lexeme) {
                    // A keyword ahead of the mark, as `try:`, begins a
                    // statement of the body, not an annotated member.
                    self.pos += 2;
                    self.put_by_annotation(&["stmt.assign"])?;
                    annotated_names.push((Value::text(&member), Value::Nil));
                    if self.on_assign() {
                        self.advance();
                        attributes.push(member);
                        Some(self.comma_value()?)
                    } else { None }
                } else {
                    let _read = self.stmt()?;
                    cannot = true;
                    None
                };
                if let Some(value) = value {
                    let place = self.gensym("attribute");
                    setup.push(Form::Write(place.clone(), Box::new(value)));
                    let word = attributes.last().expect("an attribute").clone();
                    if let Some(index) = attributes[..attributes.len() - 1].iter().position(|n| n == &word) {
                        attributes.remove(index);
                        values.remove(index + usize::from(parent.is_some()) + other_parents.len());
                    }
                    self.class_bindings.last_mut().expect("the class namespace").1.insert(word, place.clone());
                    values.push(Form::Read(place));
                }
            }
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
        self.class_bindings.pop();
        self.within = previous;
        if cannot {
            setup.truncate(before_body);
            setup.push(self.class_not_ready());
        }
        // What the body annotated, carried by the class under the table's
        // word; a body that annotated nothing leaves the class without it.
        if let (Some(word), false) = (table.strings("ext.stmt.class.annotations").first(), annotated_names.is_empty()) {
            attributes.push(word.clone());
            values.push(constant(Value::Dict(Rc::new(annotated_names))));
        }
        // The header's keywords ride along as entries under their hidden
        // names, for the building of the class to hand on.
        for (word, read) in handed_words {
            attributes.push(word);
            values.push(read);
        }
        let plan = Plan {
            name: named.clone(), answers: other_parents.len(), field_names: vec![], field_reach: vec![],
            shared_names: attributes, constant_names: vec![], methods, extends: parent.is_some(),
        };
        let declaration = Form::Class { plan: Rc::new(plan), values };
        if self.class_bindings.last().map_or(false, |(level, _)| *level == self.layers.len()) {
            let slot = self.gensym("inner_class");
            self.class_bindings.last_mut().expect("an outer class").1.insert(named, slot.clone());
            setup.push(Form::Write(slot, Box::new(declaration)));
        } else { setup.push(self.write(&named, declaration)); }
        Ok((sequence(setup), cannot))
    }

    fn class_type_parameters(&mut self) -> Res<()> {
        self.advance();
        let table = self.table;
        let mut declared = Vec::new();
        loop {
            if ["ext.stmt.function.carries", "ext.stmt.function.carries.pairs"].iter().any(|key| self.on_any(key)) { self.advance(); }
            let parameter = self.need_word("among the type parameters")?;
            if declared.contains(&parameter) { return Err(table.single("ext.stmt.function.parameters.amiss").unwrap_or_default().into()); }
            declared.push(parameter);
            if self.on_any("ext.stmt.annotation") { self.advance(); self.expr_at(0, false)?; }
            if self.on_assign() { self.advance(); self.expr(0)?; }
            if self.on_any("ext.stmt.type_params.close") { break; }
            self.need_sign(table.single("syntax.call.separator").unwrap(), "between type parameters")?;
            if self.on_any("ext.stmt.type_params.close") { break; }
        }
        self.need_sign(table.single("ext.stmt.type_params.close").unwrap(), "after the type parameters")?;
        Ok(())
    }

    fn class_decl(&mut self) -> Res<Form> {
        if self.table.flag("ext.stmt.class.this.explicit") {
            return self.class_with_receiver().map(|(form, _)| form);
        }
        let table = self.table;
        let word = self.advance().lexeme;
        let name = self.need_word("as the class name")?;
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
        let plan = Plan { name: name.clone(), answers: answers.len(), field_names, field_reach: reaches, shared_names, constant_names, methods, extends: under.is_some() };
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
                true => self.put_by_annotation(&["block.intro"])?,
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
            let body = if explicit && table.blocks == Blocks::Indented {
                r.skip_lead_word();
                if r.on_stmt_end() || r.look().shape == Shape::Open { r.body()? } else { r.stmt()? }
            } else { r.body()? };
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
                        r.distribute(began..end, &item)?
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
        if !self.on_any("block.intro") { return Err(self.bad_case()); }
        self.advance();
        self.skip_line_ends();
        if self.look().shape != Shape::Open { return Err(self.bad_case()); }
        self.advance();
        let mut arms = Vec::new();
        loop {
            self.skip_line_ends();
            if self.look().shape == Shape::Close { self.advance(); break; }
            if !self.key("ext.stmt.match.case") { return Err(self.bad_case()); }
            self.advance();
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
            let mut fits = Form::Fits { value: Box::new(Form::Read(held.clone())), test: Rc::new(pattern), slots, tuple };
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
            match self.pattern_name()? {
                CaseTest::Keep(name) => test = CaseTest::Also { test: Box::new(test), name },
                _ => return Err(self.bad_case()),
            }
        }
        Ok(test)
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
        if self.look().shape == Shape::Quote {
            let mut chars = String::new();
            while self.look().shape == Shape::Quote { chars.push_str(&self.advance().lexeme); }
            return Ok(CaseTest::Equal(Value::text(&chars)));
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
            let mut values = Vec::new();
            let mut ended = false;
            while !self.on_any("syntax.map.close") {
                if ended { return Err(self.bad_case()); }
                if self.on_any("op.pow") {
                    self.advance();
                    let capture = self.pattern_name()?;
                    if !matches!(capture, CaseTest::Keep(_)) { return Err(self.bad_case()); }
                    values.push(capture);
                    ended = true;
                } else {
                    match self.pattern_single()? {
                        CaseTest::Equal(_) | CaseTest::Pending(_) => {}
                        _ => return Err(self.bad_case()),
                    }
                    self.need_sign(table.single("syntax.map.pair").unwrap_or_default(), "between a key and its pattern")?;
                    values.push(self.pattern_choice()?);
                }
                if !self.on_any("syntax.map.separator") { break; }
                self.advance();
            }
            self.need_sign(table.single("syntax.map.close").unwrap_or_default(), "after the mapping pattern")?;
            return Ok(CaseTest::Pending(values));
        }
        if self.look().shape != Shape::Bare { return Err(self.bad_case()); }
        for (label, value) in [("literal.null", Value::Nil), ("literal.true", Value::Flag(true)), ("literal.false", Value::Flag(false))] {
            if self.key(label) { self.advance(); return Ok(CaseTest::Equal(value)); }
        }
        let binding = self.pattern_name()?;
        let mut qualified = false;
        while self.on_any("op.pipe") {
            self.advance();
            self.need_word("after the member mark")?;
            qualified = true;
        }
        if !self.on_any("syntax.call.open") {
            return Ok(if qualified { CaseTest::Pending(Vec::new()) } else { binding });
        }
        self.advance();
        let mut fields = Vec::new();
        let mut named = HashSet::new();
        while !self.on_any("syntax.call.close") {
            if self.look().shape == Shape::Bare && table.spells("stmt.assign", &self.glance(1).lexeme) {
                if !named.insert(self.advance().lexeme) { return Err(self.bad_case()); }
                self.advance();
            } else if !named.is_empty() { return Err(self.bad_case()); }
            fields.push(self.pattern_choice()?);
            if !self.on_any("syntax.call.separator") { break; }
            self.advance();
        }
        self.need_sign(table.single("syntax.call.close").unwrap_or_default(), "after the class pattern")?;
        Ok(CaseTest::Pending(fields))
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
                self.body_limb(Traps::Naught)?
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
        self.advance();
        loop {
            if self.on_any("ext.stmt.function.carries") || self.on_any("ext.stmt.function.carries.pairs") { self.advance(); }
            self.need_word("among type parameters")?;
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
        Ok(true)
    }

    fn for_stmt(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
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
            let start = if self.on_any("ext.syntax.array.spread") { self.comma_value()? }
                else { let item = self.expr(tier + 1)?; self.comma_tail(item)? };
            if !(self.look().shape == Shape::Sign && table.spells("op.range", &self.look().lexeme)) {
                // No range mark: what was read is something to walk through.
                if !table.flag("ext.stmt.for.collection") {
                    return Err("A for loop needs a range: start..end".to_string());
                }
                let source = match table.single("ext.op.comprehension.for") {
                    Some(_) => prim_call(Prim::Iterated, vec![start]),
                    None => start,
                };
                self.address_to_write(&var);
                return self.walk(source, None, var, None, place);
            }
            self.advance();
            let end = self.expr(tier + 1)?;
            (start, end)
        };
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
        // Words said about how the parameters are written, said where
        // the routine is declared.
        let mut said: Vec<Form> = Vec::new();
        while !self.sign(&close) && !self.exhausted() {
            let mut manner = if beyond { 'n' } else { 'b' };
            if bind {
                if closed { return Err(wrong()); }
                let word = self.look().lexeme.clone();
                if table.spells("ext.stmt.function.positional_only", &word) {
                    if slash || beyond || params.is_empty() { return Err(wrong()); }
                    for before in &mut manners { *before = 'p'; }
                    slash = true;
                    self.advance();
                    if !self.sign(&close) {
                        self.need_sign(table.single("syntax.call.separator").unwrap_or(""), "after the positional mark")?;
                    }
                    continue;
                }
                if table.spells("ext.stmt.function.carries.pairs", &word) {
                    closed = true;
                    manner = 'k';
                    self.advance();
                } else if table.spells("ext.stmt.function.carries", &word) || table.spells("ext.stmt.function.keyword_only", &word) {
                    if beyond { return Err(wrong()); }
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
                params.push(self.need_word("as a parameter name")?);
                if !short && self.on_any("ext.stmt.annotation") {
                    self.advance();
                    self.put_by_annotation(&["stmt.assign", "syntax.call.close", "syntax.call.separator"])?;
                } else if self.look().shape == Shape::Sign && table.spells("stmt.let.annotation", &self.look().lexeme) {
                    self.advance();
                    self.need_word("as a type name")?;
                }
            }
            if bind {
                let last = params.last().ok_or_else(wrong)?;
                if params.iter().filter(|p| *p == last).count() != 1 { return Err(wrong()); }
                match (self.on_assign(), manner) {
                    (true, 'v' | 'k') => return Err(wrong()),
                    (true, 'b') => optional = true,
                    (false, 'b') if optional => return Err(wrong()),
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
        self.taking = if bind { Some(manners) } else { None };
        Ok((params, spares, also_property, said))
    }

    /// Pass over the kind written beside a name. Its brackets shelter
    /// their contents from the marks ending the surrounding declaration.
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
        self.type_names()?;
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
                true => self.put_by_annotation(&["block.intro"])?,
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
        let program = self.routine(&name, Holds::Every, Traps::Yields, params, least, |r| {
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
        let bad = || table.single("ext.stmt.function.parameters.amiss").unwrap_or_default().to_string();
        loop {
            if self.sign(colon) { self.advance(); break; }
            if pairs { return Err(bad()); }
            if self.exhausted() { return Err("Expected lambda body".to_string()); }
            if table.spells("op.div", &self.look().lexeme) {
                if table.flag("ext.stmt.function.closes_over") {
                    if positional_mark || named_only || names.is_empty() {
                        return Err(table.single("ext.stmt.function.parameters.amiss").unwrap_or_default().into());
                    }
                    positional_mark = true;
                    for way in &mut ways { *way = 'p'; }
                }
                self.advance();
            } else {
                let many = table.spells("op.mul", &self.look().lexeme);
                let mapping = table.spells("op.pow", &self.look().lexeme);
                let mut way = if named_only { 'n' } else { 'b' };
                if many || mapping {
                    if many && named_only { return Err(bad()); }
                    self.advance();
                    named_only = true;
                    pairs = mapping;
                    way = if mapping { 'k' } else { 'v' };
                    if many && self.sign(comma) {
                        self.advance();
                        if self.sign(colon) { return Err(bad()); }
                        continue;
                    }
                    gather = Some(names.len());
                }
                let parameter = self.need_word("as a lambda parameter")?;
                if names.contains(&parameter) { return Err("Duplicate lambda parameter".to_string()); }
                names.push(parameter);
                ways.push(way);
                if self.on_assign() {
                    if many || mapping { return Err(bad()); }
                    if way == 'b' { default_seen = true; }
                    self.advance();
                    let worth = self.expr(0)?;
                    let hidden = self.gensym("spare");
                    before.push(Form::Write(hidden.clone(), Box::new(worth)));
                    spares.push((names.len() - 1, hidden));
                } else if way == 'b' && default_seen { return Err(bad()); }
            }
            if !self.sign(colon) { self.need_sign(comma, "between lambda parameters")?; }
        }
        let required = gather.unwrap_or(names.len()).saturating_sub(spares.len());
        let parameters = names.clone();
        let bind_arguments = table.flag("ext.syntax.call.bind_names");
        if bind_arguments { self.taking = Some(ways); }
        let enclosing: Vec<String> = self.layers.iter().skip(1).filter(|l| l.holds == Holds::Every).flat_map(|l| l.idents.clone()).collect();
        let mut unavailable = self.outside_lambda.clone();
        unavailable.extend(enclosing);
        let prior = std::mem::replace(&mut self.outside_lambda, unavailable);
        let mut function = self.routine(ANONYMOUS, Holds::Every, Traps::Yields, names, required, |b| {
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
            let body = b.expr(0)?;
            steps.push(body);
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
                true => self.put_by_annotation(&["block.intro"])?,
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
    fn comma_part(&mut self) -> Res<(Form, bool)> {
        let star = self.table.single("ext.op.tuple").is_some() && self.on_any("ext.stmt.unpack.rest");
        if star { self.advance(); }
        let value = self.expr(0)?;
        let gathered = if star {
            let partition = prim_call(Prim::Partition(1, Some(0)), vec![value]);
            prim_call(Prim::Apart, vec![partition, constant(Value::Small(0))])
        } else { value };
        Ok((gathered, star))
    }

    /// A target is read afresh at its turn, after the whole right hand
    /// side has been kept. Nested targets each check their own extent.
    fn distribute(&mut self, span: std::ops::Range<usize>, source: &str) -> Res<Form> {
        let bad = self.table.single("ext.stmt.unpack.amiss").unwrap_or("Invalid assignment target").to_string();
        let (mut lo, mut hi) = (span.start, span.end);
        if lo >= hi { return Err(bad); }
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
        self.pos = signs[signs.len() - 1] + 1;
        let answer = self.comma_value()?;
        let resume = self.pos;
        let name = self.gensym("given").ident.to_string();
        let mut forms = vec![self.write(&name, answer)];
        for sign in signs {
            forms.push(self.distribute(left..sign, &name)?);
            left = sign + 1;
        }
        self.pos = resume;
        Ok(Some(sequence(forms)))
    }

    fn with_annotation(&mut self, place: Form) -> Res<Form> {
        let table = self.table;
        if !matches!(&place, Form::Read(_)
            | Form::Apply(Callee::Prim(Prim::At | Prim::Of, _), _)) {
            return Err(table.single("ext.stmt.annotation.amiss").unwrap_or("Expected an assignment target").to_string());
        }
        self.advance();
        self.put_by_annotation(&["stmt.assign", "ext.stmt.annotation", "syntax.call.separator"])?;
        if self.on_assign() {
            return self.written(place, false);
        }
        Ok(match place {
            Form::Read(slot) => {
                if table.flag("ext.stmt.function.closes_over") { self.address_to_write(&slot.ident); }
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
        if let Some(write) = self.chained_places()? { return Ok(write); }
        if self.divided_at(self.pos, self.tokens.len(), "stmt.assign").is_empty()
            && !self.divided_at(self.pos, self.tokens.len(), "ext.op.tuple").is_empty() { return self.comma_value(); }
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
        let follows = |reader: &Self| reader.on_assign() && reader.glance(1).shape == Shape::Bare
            && reader.glance(2).shape == Shape::Sign && reader.table.spells("stmt.assign", &reader.glance(2).lexeme);
        if self.table.flag("ext.stmt.assign.chain") && follows(self) {
            if let Form::Read(first) = &expr {
                let mut destinations = vec![first.ident.to_string()];
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
            return self.with_annotation(expr);
        }
        if !self.on_writing() {
            return self.comma_tail(expr);
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
        let refused_slice = !plain && slice_target(&expr);
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
            Some(first)
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
                    let current = self.read(&slot.ident);
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
                let start = self.cell_of(stands_on)?;
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
                steps.push(self.write(&name, back));
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
            _ => return Err(format!("Invalid assignment target before '{}'", assign.lexeme)),
        };
        let made = if refused_slice {
            sequence(vec![prim_call(Prim::SliceRefused, Vec::new()), made])
        } else { made };
        let made = match before_bounds {
            Some(first) => sequence(vec![first, made]),
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

    /// An expression. Where a language counts a write as one, a target
    /// followed by a sign that writes is read as a write whose value is
    /// what was written — but not where the write is the whole
    /// statement, which is read as a statement.
    fn expr_at(&mut self, floor: u32, may_write: bool) -> Res<Form> {
        let table = self.table;
        if floor == 0 && self.look().shape == Shape::Bare && table.spells("ext.op.assign.expression", &self.glance(1).lexeme) {
            let word = self.advance().lexeme;
            self.advance();
            let target = self.address_to_write(&word);
            let expression = self.expr(0)?;
            return Ok(sequence(vec![Form::Write(target.clone(), Box::new(expression)), Form::Read(target)]));
        }
        let mut left = self.monadic_expr()?;
        if floor == 0 && table.spells("ext.op.assign.expression", &self.look().lexeme) {
            let Form::Read(target) = left else {
                return Err("Named expression needs a variable".to_string());
            };
            self.advance();
            let rhs = self.expr(0)?;
            let bind = self.write(&target.ident, rhs);
            return Ok(sequence(vec![bind, self.read(&target.ident)]));
        }
        if floor == 0 && may_write && table.flag("ext.op.assign.value") && self.on_writing() {
            return self.written(left, true);
        }
        loop {
            let t = self.look();
            if t.shape != Shape::Sign && t.shape != Shape::Bare {
                break;
            }
            let text = t.lexeme.clone();
            let conditional = table.strings("ext.op.if_else");
            if floor == 0 && conditional.first() == Some(&text) {
                self.advance();
                let test = self.expr(1)?;
                let end = conditional.get(1).ok_or("Conditional expression needs two words")?;
                if self.look().lexeme != *end {
                    return Err(format!("Expected '{}' in conditional expression", end));
                }
                self.advance();
                let no = self.expr(0)?;
                left = self.choose(test, left, no);
                continue;
            }
            if table.flag("ext.op.compare.chained") {
                if let Some((operation, level, words)) = self.comparison_head() {
                    if floor > level { break; }
                    let saved = self.gensym("middle");
                    let keep = Form::Write(saved.clone(), Box::new(left));
                    let links = self.comparison_tail(saved, operation, level, words)?;
                    left = sequence(vec![keep, links]);
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
                let rest = self.comparison_tail(far, following, tier, width)?;
                self.choose(test, rest, constant(Value::Flag(false)))
            }
            _ => test,
        };
        Ok(sequence(vec![put, answer]))
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
            self.advance();
            return self.monadic_expr();
        }
        if self.key("ext.stmt.yield") {
            self.advance();
            self.generator_seen = true;
            let previous_yield = std::mem::replace(&mut self.reading_yield, true);
            let from = self.key("ext.stmt.yield.from");
            if from { self.advance(); }
            let at_end = |r: &Self| r.on_stmt_end() || r.exhausted() || r.look().shape == Shape::Close
                || r.on_any("syntax.group.close") || r.on_any("syntax.array.close");
            let mut values = Vec::new();
            let mut comma = false;
            if from || !at_end(self) {
                loop {
                    values.push(self.expr(0)?);
                    if from || !self.on_any("syntax.call.separator") { break; }
                    comma = true;
                    self.advance();
                    if at_end(self) { break; }
                }
            }
            self.reading_yield = previous_yield;
            if table.flag("ext.stmt.yield.suspends") {
                let value = if comma { prim_call(Prim::MakeTuple, values) } else { values.pop().unwrap_or_else(|| constant(Value::Nil)) };
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
                fields: vec![], constants: vec![], reaches: vec![], answers: vec![],
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
            Shape::ByteQuote | Shape::Quote | Shape::Woven | Shape::Unheld => {
                let mut text = self.quotation()?;
                if table.flag("ext.lexical.string.adjacent") {
                    while matches!(self.look().shape, Shape::ByteQuote | Shape::Quote | Shape::Woven | Shape::Unheld) {
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
                self.advance();constant(Value::Wrapped(9,Rc::new(Vec::new())))
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
            if targets {
                let closes = if self.on_any("syntax.group.open") { table.single("syntax.group.close") }
                    else if self.on_any("syntax.array.open") { table.single("syntax.array.close") } else { None };
                if let Some(closes) = closes {
                    self.advance();
                    items.push(self.forget_list(true, true, Some(closes.to_string()))?);
                    if self.on_any("syntax.call.separator") { self.advance(); }
                    continue;
                }
            }
            self.unsupported_place = false;
            let named = if targets { self.deletion_place()? } else { self.expr(0)? };
            items.push(if self.unsupported_place {
                prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.stmt.del.unrun").unwrap_or_default()))])
            } else { match named {
                Form::Read(slot) => {
                    if targets && table.flag("ext.stmt.function.closes_over") {
                        let own = self.address_to_write(&slot.ident);
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
                let shared = self.address_to_write(&slot.ident.to_string());
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
                        return Ok(if calling { Form::Apply(Callee::Prim(op, Rc::from(named.as_str())), args) }
                            else { prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.builtin.set.method.unavailable").unwrap_or_default()))]) });
                    }
                    if !calling && matches!(table.prims.get(&named), Some(Prim::Octets(_))) {
                        args.push(prim_call(Prim::Raise, vec![constant(Value::text(table.single("ext.system.bytes.unready").unwrap_or("")))]));
                        return Ok(sequence(args));
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
                node = sequence(vec![save, branch]);
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
        if let Some(next) = self.ahead_in_item("ext.op.comprehension.for") {
            return self.gather_comprehension(next, &closing, mapped);
        }
        let mut value = prim_call(if mapped { Prim::MakeMap } else if family == "map" { Prim::EmptySet } else { Prim::MakeArray }, vec![]);
        while !self.sign(&closing) {
            let spreading = self.on_any(if mapped { "ext.syntax.map.spread" } else { "ext.syntax.array.spread" });
            if spreading { self.advance(); }
            let mut item = self.expr(0)?;
            if mapped && !spreading {
                self.need_sign(self.table.single("syntax.map.pair").unwrap(), "between the key and its value")?;
                let right = self.expr(0)?;
                item = prim_call(Prim::Couple, vec![item, right]);
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
        while !self.on_any("ext.op.comprehension.in") && !self.exhausted() { self.advance(); }
        self.advance();
        let begins = self.pos;
        let source = self.expr(1)?;
        let ends = self.pos;
        let parameter = self.gather_name("first_source");
        let previous = self.source_before.replace((begins, ends, parameter.clone()));
        let routine = self.routine("<generator>", Holds::Every, Traps::Yields, vec![parameter], 1, |r| {
            r.pos = clause;
            let before = r.gather_names.len();
            let body = r.gather_tail(head, "", false)?;
            r.gather_names.truncate(before);
            r.need_sign(end, "after a generator expression")?;
            r.generator_seen = true;
            Ok(body)
        })?;
        self.source_before = previous;
        Ok(Form::Apply(Callee::Code(Box::new(routine)), vec![prim_call(Prim::Walked, vec![source])]))
    }

    fn gather_comprehension(&mut self, first_for: usize, end: &str, dictionary: bool) -> Res<Form> {
        if self.table.flag("ext.stmt.function.closes_over") {
            let routine = self.routine("<gathering>", Holds::Every, Traps::Yields, Vec::new(), 0,
                |reader| reader.gather_in_scope(first_for, end, dictionary))?;
            return Ok(Form::Apply(Callee::Code(Box::new(routine)), Vec::new()));
        }
        self.gather_in_scope(first_for, end, dictionary)
    }

    fn gather_in_scope(&mut self, first_for: usize, end: &str, dictionary: bool) -> Res<Form> {
        let expression_at = self.pos;
        self.pos = first_for;
        let old_names = self.gather_names.len();
        let name = self.gather_name("gathered");
        let empty = prim_call(if dictionary { Prim::MakeMap } else if self.table.flag("ext.syntax.set") && self.table.spells("syntax.map.close", end) { Prim::EmptySet } else { Prim::MakeArray }, Vec::new());
        let start = self.write(&name, empty);
        let work = self.gather_tail(expression_at, &name, dictionary)?;
        self.gather_names.truncate(old_names);
        self.need_sign(end, "to finish a comprehension")?;
        let answer = self.read(&name);
        Ok(sequence(vec![start, work, answer]))
    }

    /// Build the clauses outside the expression they govern. Each walk
    /// owns its names; the first source still sees the names outside it.
    fn gather_tail(&mut self, expression_at: usize, answer: &str, dictionary: bool) -> Res<Form> {
        if self.on_any("ext.op.comprehension.if") {
            self.advance();
            let condition = self.expr(1)?;
            let accepted = self.gather_tail(expression_at, answer, dictionary)?;
            return Ok(self.choose(condition, accepted, constant(Value::Nil)));
        }
        if self.on_any("ext.op.comprehension.async") {
            self.advance();
            if !self.on_any("ext.op.comprehension.for") { return Err("Expected a walk after the asynchronous word".into()); }
            let words = self.table.single("ext.op.comprehension.async.unavailable").unwrap_or("Asynchronous walks are not provided");
            let refusal = prim_call(Prim::Raise, vec![constant(Value::text(words))]);
            let read = self.gather_tail(expression_at, answer, dictionary)?;
            return Ok(sequence(vec![refusal, read]));
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
            let so_far = self.read(answer);
            let enlarged = prim_call(Prim::ExtendLiteral(dictionary, spread), vec![so_far, term]);
            return Ok(self.write(answer, enlarged));
        }
        self.advance();
        let grouped = self.on_any("syntax.group.open");
        if grouped { self.advance(); }
        let mut targets = Vec::new();
        let mut taken_apart = false;
        loop {
            let spelled = self.look().lexeme.clone();
            let target = self.monadic_expr()?;
            targets.push(match target {
                Form::Read(_) => Some(spelled),
                Form::Apply(Callee::Prim(Prim::At, _), _) => None,
                _ => return Err("Expected a name or indexed place as a comprehension target".into()),
            });
            if !self.on_any("syntax.call.separator") { break; }
            self.advance();
            taken_apart = true;
            if self.on_any("ext.op.comprehension.in") || self.on_any("syntax.group.close") { break; }
        }
        if grouped { self.need_sign(self.table.single("syntax.group.close").unwrap(), "after the target")?; }
        if !self.on_any("ext.op.comprehension.in") { return Err("Expected the word before a comprehension source".into()); }
        self.advance();
        let unavailable = targets.iter().any(Option::is_none);
        let source = match self.source_before.clone().filter(|(at, _, _)| *at == self.pos) {
            Some((_, end, parameter)) => { self.pos = end; self.read(&parameter) }
            None => self.expr(1)?,
        };
        let walks = self.table.flag("ext.stmt.yield.suspends");
        let source_name = self.gather_name("gather_source");
        let hold = self.write(&source_name, prim_call(if walks { Prim::Walked } else { Prim::Iterated }, vec![source]));
        let cursor = self.gather_name("gather_cursor");
        let begin = self.write(&cursor, constant(Value::Small(0)));
        let bag = self.read(&source_name);
        let index = self.read(&cursor);
        let test = if walks { prim_call(Prim::MoreYet, vec![bag, index]) }
            else { prim_call(Prim::Lt, vec![index, prim_call(Prim::Length, vec![bag])]) };
        let bag = self.read(&source_name);
        let index = self.read(&cursor);
        let mut item = prim_call(if walks { Prim::AtHand } else { Prim::At }, vec![bag, index]);
        if taken_apart { item = prim_call(Prim::CheckUnpack(targets.len()), vec![item]); }
        let item_name = self.gather_name("gather_item");
        let mut body = vec![self.write(&item_name, item)];
        for (part, original) in targets.into_iter().enumerate() {
            let private = self.gather_name("gather_binding");
            let mut value = self.read(&item_name);
            if taken_apart { value = prim_call(Prim::At, vec![value, constant(Value::Small(part as i64))]); }
            body.push(self.write(&private, value));
            if let Some(original) = original { self.gather_names.push((original, private)); }
        }
        body.push(self.gather_tail(expression_at, answer, dictionary)?);
        let before = self.read(&cursor);
        let onward = self.write(&cursor, prim_call(Prim::Plus, vec![before, constant(Value::Small(1))]));
        let cycle = Form::Cycle { test: Box::new(test), body: Box::new(sequence(body)), step: Some(Box::new(onward)), after: false, otherwise: None };
        let mut work = vec![hold, begin, cycle];
        if unavailable {
            let words = self.table.single("ext.op.comprehension.target.unavailable").unwrap_or("Indexed comprehension targets are not provided");
            work.insert(0, prim_call(Prim::Raise, vec![constant(Value::text(words))]));
        }
        Ok(sequence(work))
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

    fn args(&mut self, close_key: &str, sep_key: &str) -> Res<Vec<Form>> {
        if let Some(at) = self.ahead_in_item("ext.op.comprehension.for") {
            let end = self.table.single(close_key).unwrap().to_string();
            return Ok(vec![self.generator_comprehension(at, &end)?]);
        }
        let close = self.table.single(close_key).unwrap().to_string();
        let sep = self.table.single(sep_key).map(str::to_string);
        let mut items = Vec::new();
        let mut named_values = Vec::new();
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
                if bind { tag = Some(Value::text(&self.look().lexeme)); }
                self.pos += 2;
            } else if bind && matches!(self.look().shape, Shape::Bare | Shape::Sign) {
                let word = &self.look().lexeme;
                if self.table.spells("ext.syntax.call.spread.pairs", word) { tag = Some(Value::Flag(true)); }
                else if self.table.spells("ext.syntax.call.spread", word) { tag = Some(Value::Flag(false)); }
                if tag.is_some() { self.advance(); }
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

    fn named_call(&mut self, name: &str, args: Vec<Form>) -> Res<Form> {
        // A name the program has bound is called as that name, in front
        // of any builtin word spelled the same, where the table says so.
        if self.table.flag("ext.syntax.names.shadow_builtins") && self.named_in_program.iter().any(|word| word == name) {
            let target = self.read(name);
            return Ok(invoke(target, args));
        }
        match self.table.prims.get(name).copied().filter(|op| !matches!(op, Prim::SetCall(1..=17))) {
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
            self.layers.push(Layer { comprehension: false, borrowed: Vec::new(), holds: Holds::Every, idents: Vec::new(), formals: Vec::new(), formal_slots: Vec::new(), rpn: true, aliases: Vec::new() });
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
            let program = Routine { qualification: String::new(), doc: None, generator: false, local_defaults: Vec::new(), gather_from: None, ident: name, least: 0, formals: params, formal_kinds: Vec::new(), taking: None, formal_slots: param_slots, idents: scope.idents, frameless: false, written_in: self.written_in.clone(), within: None, declared_on: 0, traps: Traps::Yields, carried: Vec::new(), body: sequence(s) };
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
        Form::Write(_, value) | Form::OnLine(_, value) | Form::Muted(value) | Form::Silenced(value) => borrows_enclosing(value, names),
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
