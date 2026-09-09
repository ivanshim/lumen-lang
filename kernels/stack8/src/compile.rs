// Compiling: tokens to words in one syntax-directed pass, then a
// peephole over the words of each routine. Expressions come out in
// post-order; a statement becomes its words with conditional skips
// around them; a function or a program value becomes a routine pushed as
// a constant. Names become cells here.
//
// Every construct is first a shape of the five floor words:
//   a leap            Const false; Skip target
//   a loop            Const false; Skip test; top: body; test: not-cond; Skip top
//                     (tested at the bottom; a comparison is flipped to its
//                     complement, anything else gets a Not)
//   a call            args; Read f; Act Invoke
//   a result          Write #result after each expression statement,
//                     Read #result at the end; return leaps to the end;
//                     a function that only ever returns has no cell for it
//   a and b           a; Write #t; Read #t; Skip over; b; Write #t;
//                     over: Read #t; Act AsBool
//   arr[i] = v        i; v; Read arr moving; Act replace; Write arr
//   dup, swap, ...    writes and reads of spare cells
//   [ 1 2 3 ]         Const fence; 1 2 3; Act collect
// and then the peephole fuses `Read a; b; Act op` into Dyad, `Read x;
// Const k; Act add; Write x` into Bump, and `Read a; b; Act cmp; Skip`
// into SkipCmp.

use std::collections::HashMap;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::lang::{Lang, Brackets, Blocks, Complaint};
use crate::arith;
use crate::lex::{Shape, Token};
use crate::value::{Reach, Value};
use crate::code::{Operand, Builtin, Action, Routine, Cell, Instr, Plan, Attempt, Taking};

/// The global names, each with a slot.
#[derive(Default)]
pub struct Registry {
    index: HashMap<String, usize>,
    pub idents: Vec<String>,
    /// The line the reading had reached when it stopped, for a language
    /// that tells such a stopping in its own words to name.
    pub stopped_at: usize,
    /// Whether the reading stopped over a thing the language calls a
    /// fault of the run rather than a program it could not read: the
    /// words are the same either way, but the kind is not.
    pub stopped_fatally: bool,
    /// Words the language has about how a program is written rather than
    /// about what it does. They are found while reading and said before
    /// the run, since that is when the reference says them.
    pub said_while_reading: Vec<(Complaint, String, u32)>,
    /// Where each bag of members a class may take in begins, by name:
    /// the token just past the mark that opens its body. Its members are
    /// read again wherever a class takes them in.
    pub bags: std::collections::HashMap<String, usize>,
    /// Which parameters of which routines take a cell rather than a
    /// value, and which routines give a cell back. Kept here because
    /// text read while the run goes is a piece of the same program and
    /// must know what the whole of it declared.
    pub shared_args: HashMap<String, Vec<bool>>,
    /// What each routine calls its parameters, so that a language may
    /// name the one it is speaking of.
    pub arg_names: HashMap<String, Vec<String>>,
    pub gives_back: std::collections::HashSet<String>,
}

impl Registry {
    pub fn slot(&mut self, name: &str) -> usize {
        if let Some(&slot) = self.index.get(name) {
            return slot;
        }
        self.index.insert(name.to_string(), self.idents.len());
        self.idents.push(name.to_string());
        self.idents.len() - 1
    }
}

/// An open loop: where continue goes once known, and the jumps waiting.
struct Cycle {
    began: usize,
    restart: Option<usize>,
    resumes: Vec<usize>,
    leaves: Vec<usize>,
}

/// The program being assembled.
struct Piece {
    outermost: bool,
    ident: String,
    idents: Vec<String>,
    /// Which slots a bare block declared: found from inside that block
    /// only, never once it has closed.
    declared: Vec<bool>,
    scopes: Vec<Vec<usize>>,
    /// Names a `global` statement bound to the global of that name, and
    /// names a `static` statement bound to a hidden global.
    globals: Vec<(String, String)>,
    /// Where the last parts of the open try statements begin: each runs
    /// before a return, a break or a continue leaves them.
    lasts: Vec<usize>,
    cycles: Vec<Cycle>,
    escapes: Vec<usize>,
    /// Whether an expression statement stored into the result slot; a
    /// function without one needs neither the slot nor its prologue.
    result_touched: bool,
    generator: bool,
    /// Which line the last marker in this unit named, so that a run of
    /// statements on one line marks it once.
    line: u32,
    instrs: Vec<Instr>,
}

pub struct Compiler<'a> {
    lang: &'a Lang,
    tokens: &'a [Token],
    pos: usize,
    registry: &'a mut Registry,
    pieces: Vec<Piece>,
    counter: usize,
    yield_operand: bool,
    awkward_place: bool,
    for_binding: Option<(String, usize)>,
    comprehension_names: Vec<(String, String)>,
    /// The class being read, and what it stands on: what `self` and
    /// `parent` mean inside a method.
    within: Option<(String, Option<String>)>,
    /// Which parameters of each program take a name's own cell rather
    /// than a copy of what it holds, found before anything is read.
    shared_args: HashMap<String, Vec<bool>>,
    arg_names: HashMap<String, Vec<String>>,
    /// The routines the text declares as giving back a cell rather than
    /// a copy, so that a call of one is known to have a cell to share.
    gives_back: std::collections::HashSet<String>,
    /// The parameters of the method just read that name properties too.
    promoted: Vec<String>,
    /// The line the routine now being read was written on, which a
    /// fault raised on the way into it names.
    declared_at: u32,
    /// The slots the routine being put together fills from what it
    /// carried away, while its parameters are being read.
    carrying: Vec<usize>,
    /// How many lines stand before the program's own text.
    before: u32,
    /// Where each key of the index chain just read begins, so that a
    /// write to a place within a place can take the keys apart and work
    /// each of them out exactly once.
    keyed: Vec<usize>,
    /// The file this text came out of, where it was read while the run
    /// was going, so that every program built from it carries it.
    written_in: Option<Rc<str>>,
    /// Whether this text was read in while the run was already going,
    /// as text handed to the word that reads text is, and a file asked
    /// for part way through. The whole of a program read that way is a
    /// piece of a run in progress, which a `static` written at the top
    /// of it is told by.
    read_in: bool,
    /// The names a `static` at the top of text read in has already
    /// spoken for. Such a statement binds nothing that would show a
    /// name said twice, so the names are kept here to be counted.
    read_statics: Vec<String>,
    /// Where the value a store is to write is already waiting, which a
    /// taking-apart sets before each of its places.
    waiting: Option<String>,
    /// How far a compound write steps what the place already holds,
    /// where the step is the write's own and no value follows the sign:
    /// what `++` and `--` mean.
    stepping: Option<i64>,
    /// Where the value a place held before a compound write is to be
    /// kept, so that a step may give back what stood there before it.
    stood: Option<String>,
    /// Whether each routine being read gives back a cell rather than a
    /// copy, the innermost last, so that what it answers with is made a
    /// cell where it should be.
    giving_cells: Vec<bool>,
    /// The class each parameter of the routine being read is declared to
    /// take, gathered as the parameters are read and taken by the
    /// routine they belong to.
    formal_kinds: Vec<Option<Rc<str>>>,
    uncarried: Vec<String>,
    parameter_rules: Option<Vec<u8>>,
}

type Res<T> = Result<T, String>;

/// What a run of postfix instrs is part of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Span {
    /// A block or the top level: ends at the block's end.
    Block,
    /// A program value: ends at its closing bracket, indented freely.
    Routine,
    /// One line: ends at the line end.
    Line,
}

const RESULT_CELL: &str = "#result";
const TEMP_CELL: &str = "#t";
/// What a class body gathers as it is read: its properties and how far
/// each may be reached from, the values it keeps for itself, its
/// constants and its methods.
struct Members {
    fields: Vec<(String, Vec<Instr>)>,
    shared: Vec<(String, Vec<Instr>)>,
    constants: Vec<(String, Vec<Instr>)>,
    methods: Vec<(String, Rc<Routine>)>,
    reaches: Vec<Reach>,
}

/// What a routine written where a value stands is called, since it
/// is bound to no name of its own.
const ANONYMOUS: &str = "{closure}";

const SPARE_CELLS: [&str; 3] = ["#a", "#b", "#c"];

/// `before` is how many lines were put before the program's own text,
/// which the host knows and a line named in a complaint must not count.
pub fn compile(tokens: &[Token], lang: &Lang, table: &mut Registry, before: u32) -> Res<Rc<Routine>> {
    compile_within(tokens, lang, table, before, None, None, None, false)
}

/// The same, told besides which file the text came out of. Only text
/// read in while the run was already going is assembled this way, so a
/// statement whose meaning turns on that may see it.
pub fn compile_from(tokens: &[Token], lang: &Lang, table: &mut Registry, before: u32, written_in: Option<Rc<str>>) -> Res<Rc<Routine>> {
    compile_within(tokens, lang, table, before, written_in, None, None, true)
}

/// The same, save that the text may be read as standing inside a
/// routine already running: the names it already has are given here, and
/// a name the text reads or writes means that one. Names of its own are
/// added on the end, so the frame it runs in need only be made longer.
pub fn compile_within(
    tokens: &[Token],
    lang: &Lang,
    table: &mut Registry,
    before: u32,
    written_in: Option<Rc<str>>,
    inside: Option<Vec<String>>,
    within: Option<(String, Option<String>)>,
    read_in: bool,
) -> Res<Rc<Routine>> {
    let alone = inside.is_none();
    let already = inside.unwrap_or_default();
    let top = Piece {
        outermost: alone,
        ident: "<program>".to_string(),
        declared: vec![false; already.len()],
        idents: already,
        scopes: Vec::new(),
        globals: Vec::new(),
        lasts: Vec::new(),
        cycles: Vec::new(),
        escapes: Vec::new(),
        result_touched: false,
            generator: false,
        line: 0,
        instrs: Vec::new(),
    };
    // Text read as standing inside a routine is a piece of a program
    // already read: what that program declared about cells stands here.
    let (mut shared_args, mut arg_names, mut gives_back) = shared_parameters(tokens, lang);
    if !alone {
        for (name, marks) in &table.shared_args {
            shared_args.entry(name.clone()).or_insert_with(|| marks.clone());
        }
        for (name, called) in &table.arg_names {
            arg_names.entry(name.clone()).or_insert_with(|| called.clone());
        }
        gives_back.extend(table.gives_back.iter().cloned());
    }
    let mut a = Compiler { yield_operand: false, awkward_place: false, for_binding: None, uncarried: Vec::new(), lang, tokens, pos: 0, registry: table, pieces: vec![top], counter: 0, comprehension_names: Vec::new(), declared_at: 0, carrying: Vec::new(), within, shared_args, arg_names, gives_back, promoted: Vec::new(), before, keyed: Vec::new(), written_in, read_in, read_statics: Vec::new(), waiting: None, stepping: None, stood: None, giving_cells: Vec::new(), formal_kinds: Vec::new(), parameter_rules: None };
    if lang.rpn {
        if let Err(said) = a.rpn_body(&[], Span::Block) {
            a.registry.stopped_at = a.look().row;
            return Err(said);
        }
        if !a.exhausted() {
            a.registry.stopped_at = a.look().row;
            return Err(format!("Unexpected '{}'", a.look().lexeme));
        }
    } else {
        // Top-level function definitions may be lifted to the front, so
        // a call above its function finds it (ext.stmt.function.hoisted).
        let mut lifted: Vec<Instr> = Vec::new();
        a.skip_seps();
        while !a.exhausted() {
            let defines = lang.hoisted && a.on_keyword(&lang.function_words);
            let from = a.mark();
            // Where the reading stops, the line it had reached is kept,
            // so that a language with a word for such a stopping may
            // name the line as it names any other.
            if let Err(said) = a.stmt() {
                a.registry.stopped_at = a.look().row;
                return Err(said);
            }
            if defines {
                lifted.extend(a.piece().instrs.drain(from..));
            }
            a.skip_seps();
        }
        if !lifted.is_empty() {
            let end = a.mark();
            for at in a.piece().escapes.clone() {
                a.patch_jump(at, end);
            }
            a.piece().escapes.clear();
            let rest = std::mem::take(&mut a.piece().instrs);
            let delta = lifted.len() as i64;
            a.piece().instrs = lifted;
            a.piece().instrs.extend(relocated(rest, delta));
        }
    }
    let end = a.mark();
    for at in a.piece().escapes.clone() {
        a.patch_jump(at, end);
    }
    if alone {
        a.registry.shared_args = a.shared_args.clone();
        a.registry.arg_names = a.arg_names.clone();
        a.registry.gives_back = a.gives_back.clone();
    }
    // What the language had to say about how the program is written is
    // said before the program runs, which is when the reference says it.
    let said = std::mem::take(&mut a.registry.said_while_reading);
    if !said.is_empty() {
        let mut head: Vec<Instr> = Vec::new();
        let spare = Cell { ident: Rc::from(SPARE_CELLS[0]), near: Vec::new(), far: a.registry.slot(SPARE_CELLS[0]), moving: false };
        for (kind, words, row) in said {
            head.push(Instr::Line(row));
            head.push(Instr::Act(Action::Remark(kind, Rc::from(words.as_str())), 0));
            head.push(Instr::Write(spare.clone()));
        }
        let rest = std::mem::take(&mut a.piece().instrs);
        let moved = head.len() as i64;
        a.piece().instrs = head;
        let shifted = relocated(rest, moved);
        a.piece().instrs.extend(shifted);
    }
    let unit = a.pieces.pop().expect("the top unit");
    Ok(Rc::new(Routine { rest_at: None, ident: unit.ident, formals: Vec::new(), formal_kinds: Vec::new(), parameter_rules: None, least: 0, idents: unit.idents, returns_value: false, body_of_all: true, written_in: a.written_in.clone(), within: None, declared_on: 0, carried: Vec::new(), held: Vec::new(), instrs: Rc::new(peephole(unit.instrs)) }))
}

impl<'a> Compiler<'a> {
    // ---------- tokens ----------

    fn look(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn look_ahead(&self, ahead: usize) -> &Token {
        &self.tokens[(self.pos + ahead).min(self.tokens.len() - 1)]
    }

    fn take(&mut self) -> Token {
        let tok = self.look().clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn exhausted(&self) -> bool {
        self.look().shape == Shape::Finish
    }

    fn at_symbol(&self, text: &str) -> bool {
        self.look().is_lexeme(Shape::Sign, text)
    }

    /// A delimiter may be a word or a symbol.
    fn at_lexeme(&self, text: &str) -> bool {
        let t = self.look();
        matches!(t.shape, Shape::Sign | Shape::Instr) && t.lexeme == text
    }

    /// Whether a block opens here, past any line ends before it: a
    /// method may be written with its body on the line after its name.
    fn block_ahead(&self) -> bool {
        let mut ahead = 0;
        while self.look_ahead(ahead).shape == Shape::LineEnd {
            ahead += 1;
        }
        let t = self.look_ahead(ahead);
        t.shape == Shape::Sign && self.lang.block_opens.iter().any(|o| *o == t.lexeme)
    }

    fn on_any(&self, list: &[String]) -> bool {
        list.iter().any(|x| self.at_lexeme(x))
    }

    fn on_keyword(&self, list: &[String]) -> bool {
        self.look().shape == Shape::Instr && Lang::spells(list, &self.look().lexeme)
    }

    fn on_sep(&self) -> bool {
        let t = self.look();
        t.shape == Shape::LineEnd || (t.shape == Shape::Sign && self.lang.ends_stmt(&t.lexeme))
    }

    fn skip_seps(&mut self) {
        while !self.exhausted() && self.on_sep() {
            self.take();
        }
    }

    fn want_sign(&mut self, text: &str, why: &str) -> Res<()> {
        if !self.at_symbol(text) {
            return Err(format!("Expected '{}' {}, got '{}'", text, why, self.look().lexeme));
        }
        self.take();
        Ok(())
    }

    fn want_name(&mut self, why: &str) -> Res<String> {
        if self.look().shape != Shape::Instr {
            return Err(format!("Expected identifier {}, got '{}'", why, self.look().lexeme));
        }
        Ok(self.take().lexeme)
    }

    fn want_lexeme(&mut self, text: &str) -> Res<()> {
        if !self.at_lexeme(text) {
            return Err(format!("Expected '{}' to close a block, got '{}'", text, self.look().lexeme));
        }
        self.take();
        Ok(())
    }

    fn expect_closer(&mut self) -> Res<()> {
        if !self.on_any(&self.lang.block_closes) {
            return Err(format!("Expected '{}' to close a block, got '{}'", self.lang.block_closes[0], self.look().lexeme));
        }
        self.take();
        Ok(())
    }

    fn expect_intro(&mut self) -> Res<()> {
        if !self.on_any(&self.lang.block_intros) {
            return Err(format!("Expected '{}', got '{}'", self.lang.block_intros[0], self.look().lexeme));
        }
        self.take();
        Ok(())
    }

    fn skip_intro(&mut self) {
        if self.on_any(&self.lang.block_intros) {
            self.take();
        }
    }

    // ---------- instrs ----------

    fn piece(&mut self) -> &mut Piece {
        self.pieces.last_mut().expect("an open unit")
    }

    fn put(&mut self, word: Instr) -> usize {
        let unit = self.piece();
        unit.instrs.push(word);
        unit.instrs.len() - 1
    }

    fn mark(&mut self) -> usize {
        self.piece().instrs.len()
    }

    fn constant(&mut self, v: Value) {
        self.put(Instr::Const(v));
    }

    fn act(&mut self, op: Action, argc: usize) {
        self.put(Instr::Act(op, argc));
    }

    /// A conditional jump to be patched.
    fn skip(&mut self) -> usize {
        self.put(Instr::Skip(0))
    }

    /// An unconditional jump to be patched: the index of its Unless.
    fn leap(&mut self) -> usize {
        self.constant(Value::Flag(false));
        self.skip()
    }


    /// A jump back to the loop's top when the test just assembled holds:
    /// the test is turned into its negation, a comparison flipped to its
    /// complement and anything else given a Not, so an Unless goes back.
    fn loop_back(&mut self, top: usize) {
        let flipped = match self.piece().instrs.last() {
            Some(Instr::Act(Action::Lt, 2)) => Some(Action::Ge),
            Some(Instr::Act(Action::Ge, 2)) => Some(Action::Lt),
            Some(Instr::Act(Action::Gt, 2)) => Some(Action::Le),
            Some(Instr::Act(Action::Le, 2)) => Some(Action::Gt),
            Some(Instr::Act(Action::Eq, 2)) => Some(Action::Ne),
            Some(Instr::Act(Action::Ne, 2)) => Some(Action::Eq),
            _ => None,
        };
        match flipped {
            Some(op) => *self.piece().instrs.last_mut().expect("a word") = Instr::Act(op, 2),
            None => self.act(Action::Not, 1),
        }
        self.put(Instr::Skip(top));
    }

    fn land(&mut self, at: usize) {
        let here = self.mark();
        self.patch_jump(at, here);
    }

    fn patch_jump(&mut self, at: usize, to: usize) {
        match &mut self.piece().instrs[at] {
            Instr::Depart { to: target, .. } => *target = to,
            word => *word = Instr::Skip(to),
        }
    }

    fn departure(&mut self, cycle: Option<usize>) -> usize {
        if self.lang.catch_as.is_empty() { return self.leap(); }
        self.put(Instr::Depart { to: 0, cycle })
    }

    /// A jump to the end of the unit, patched when it closes.
    fn escape(&mut self) {
        let at = self.departure(None);
        self.piece().escapes.push(at);
    }

    /// The slot a name has outside every bare block: none at the top
    /// level, where such names are global.
    fn unblocked(unit: &Piece, name: &str) -> Option<usize> {
        (0..unit.idents.len()).rev().find(|&s| unit.idents[s] == name && !unit.declared[s])
    }

    /// Where a name is read: every slot of that name from the innermost
    /// open block outward, then the one outside the blocks, then the
    /// global. A closed block's slot is never found.
    /// The global a name stands for in this unit, by `global` or `static`.
    fn global_cell(&mut self, name: &str) -> Option<Cell> {
        let unit = self.pieces.last().expect("a unit");
        let target = unit.globals.iter().rev().find(|(n, _)| n == name)?.1.clone();
        Some(Cell { ident: Rc::from(name), near: Vec::new(), far: self.registry.slot(&target), moving: false })
    }

    fn cell_to_read(&mut self, name: &str, moving: bool) -> Cell {
        let renamed = self.comprehension_names.iter().rev().find(|(n, _)| n == name).map(|(_, own)| own.clone());
        let name = renamed.as_deref().unwrap_or(name);
        if let Some(cell) = self.global_cell(name) {
            return Cell { moving, ..cell };
        }
        let global = self.registry.slot(name);
        let unit = self.pieces.last().expect("a unit");
        let mut locals = Vec::new();
        for block in unit.scopes.iter().rev() {
            locals.extend(block.iter().rev().filter(|&&s| unit.idents[s] == name).copied());
        }
        locals.extend(Self::unblocked(unit, name));
        Cell { ident: Rc::from(name), near: locals, far: global, moving }
    }

    /// Where a name is written: the innermost slot of that name in the
    /// current block or function, made if there is none. At the top level
    /// a name outside every block is global; inside a block it is the
    /// block's own, forgotten on leaving.
    fn cell_to_write(&mut self, name: &str) -> Cell {
        let renamed = self.comprehension_names.iter().rev().find(|(n, _)| n == name).map(|(_, own)| own.clone());
        let name = renamed.as_deref().unwrap_or(name);
        if let Some(cell) = self.global_cell(name) {
            return cell;
        }
        let global = self.registry.slot(name);
        let unit = self.pieces.last_mut().expect("a unit");
        if unit.outermost && unit.scopes.is_empty() {
            return Cell { ident: Rc::from(name), near: Vec::new(), far: global, moving: false };
        }
        let found = match unit.scopes.last() {
            Some(block) => block.iter().rev().find(|&&s| unit.idents[s] == name).copied(),
            None => Self::unblocked(unit, name),
        };
        let slot = found.unwrap_or_else(|| {
            unit.idents.push(name.to_string());
            unit.declared.push(!unit.scopes.is_empty());
            let s = unit.idents.len() - 1;
            if let Some(block) = unit.scopes.last_mut() {
                block.push(s);
            }
            s
        });
        Cell { ident: Rc::from(name), near: vec![slot], far: global, moving: false }
    }

    fn refuse_uncarried(&mut self, name: &str) -> bool {
        if !self.uncarried.iter().any(|n| n == name) || !self.cell_to_read(name, false).near.is_empty() {
            return false;
        }
        if let Some(told) = self.lang.lambda_enclosing.clone() {
            self.constant(Value::text(&told));
            self.act(Action::Builtin(Builtin::Raise, Rc::from("lambda")), 1);
            return true;
        }
        false
    }

    fn read(&mut self, name: &str) {
        if self.refuse_uncarried(name) { return; }
        // The name a language gives the line it is written on stands
        // for that line itself, known while assembling.
        if self.lang.line_binding.as_deref() == Some(name) {
            let row = (self.look().row as u32).saturating_sub(self.before);
            self.constant(Value::Small(row as i64));
            return;
        }
        // The routine a piece is written in, the class that routine
        // belongs to, and the two written together: all known while the
        // program is put together, so each stands for what it names.
        let unit = self.piece();
        let routine = match unit.outermost {
            true => String::new(),
            false => unit.ident.clone(),
        };
        let within = self.within.as_ref().map(|(named, _)| named.clone()).unwrap_or_default();
        for (binding, said) in [
            (&self.lang.routine_binding, routine.clone()),
            (&self.lang.class_binding, within.clone()),
            (&self.lang.method_binding, match within.is_empty() {
                true => routine,
                false => format!("{}::{}", within, routine),
            }),
        ] {
            if binding.as_deref() == Some(name) {
                self.constant(Value::text(&said));
                return;
            }
        }
        self.own_place(name);
        let slot = self.cell_to_read(name, false);
        self.put(Instr::Read(slot));
    }

    fn read_taking(&mut self, name: &str) {
        self.own_place(name);
        let slot = self.cell_to_read(name, true);
        self.put(Instr::Read(slot));
    }

    /// A variable a routine only ever reads is given a place of the
    /// routine's own all the same (ext.stmt.function.own_names). The
    /// place holds nothing, and reading a place holding nothing falls
    /// through to the outermost binding as it always did, so nothing
    /// the program can see is changed by the place being there. What it
    /// is for is text read in while the run goes: such text is a piece
    /// of the routine that read it, and a write it makes to one of that
    /// routine's names needs a place in that routine's frame to land
    /// in. Only names the language marks as variables are given one:
    /// what wears no mark names a constant or a class, and the places
    /// the assembler makes for itself are none of the program's.
    fn own_place(&mut self, name: &str) {
        if !self.lang.own_names {
            return;
        }
        let marked = self.lang.sigil.map_or(true, |mark| name.starts_with(mark));
        let unit = self.pieces.last_mut().expect("a unit");
        if unit.outermost || !marked || unit.idents.iter().any(|n| n == name) {
            return;
        }
        unit.idents.push(name.to_string());
        unit.declared.push(false);
    }

    /// The name of an array about to be written into, addressed as the
    /// write it is: a name first met on the left of a write belongs to
    /// the unit it is written in, and every later reading of it must
    /// find the same cell.
    fn read_to_rewrite(&mut self, name: &str) {
        let mut slot = self.cell_to_write(name);
        slot.moving = true;
        self.put(Instr::Read(slot));
    }

    /// The store after such a load, addressed the same way.
    fn rewritten(&mut self, name: &str) {
        let slot = self.cell_to_write(name);
        self.put(Instr::Write(slot));
    }

    fn write(&mut self, name: &str) {
        let slot = self.cell_to_write(name);
        self.put(Instr::Write(slot));
    }

    /// The store after a taking load: addressed like the load, so the
    /// hole it left is found wherever the value was.
    fn restore(&mut self, name: &str) {
        let slot = self.cell_to_read(name, false);
        self.put(Instr::Write(slot));
    }

    /// The program a call by name reaches. Where a language names the
    /// result after the function, a call of the function's own name inside
    /// it is the global program, not the result being built.
    fn read_callee(&mut self, name: &str) {
        if self.refuse_uncarried(name) { return; }
        let own = self.lang.named_result && self.piece().ident == name && !self.piece().outermost;
        let mut slot = self.cell_to_read(name, false);
        if own {
            slot.near.clear();
        }
        self.put(Instr::Read(slot));
    }

    /// A hidden name no program can spell, new each time.
    fn gensym(&mut self, purpose: &str) -> String {
        self.counter += 1;
        format!("#{}{}", purpose, self.counter)
    }

    /// Discard the top of the stack. What is discarded is let go at
    /// once rather than put in a spare slot: a slot would hold the value
    /// alive until the next statement wrote over it, and a program
    /// asking how much room it holds would be told of a value it had
    /// already finished with.
    fn discard(&mut self) {
        self.put(Instr::Shed);
    }

    fn enter_cycle(&mut self, again: Option<usize>) {
        let began = self.mark();
        self.piece().cycles.push(Cycle { began, restart: again, resumes: Vec::new(), leaves: Vec::new() });
    }

    fn leave_cycle(&mut self, again: usize) {
        let lp = self.piece().cycles.pop().expect("an open loop");
        for at in lp.resumes {
            self.patch_jump(at, again);
        }
        for at in lp.leaves {
            self.land(at);
        }
    }

    fn complete_cycle(&mut self, again: usize) -> Res<()> {
        if !self.lang.loop_else { self.leave_cycle(again); return Ok(()); }
        let cycle = self.piece().cycles.pop().expect("an open loop");
        for at in cycle.resumes { self.patch_jump(at, again); }
        self.skip_seps();
        if self.on_keyword(&self.lang.else_words) {
            self.take();
            self.body()?;
        }
        for at in cycle.leaves { self.land(at); }
        Ok(())
    }

    /// How many loops a break or continue leaves: the number after the
    /// word when the definition allows one (ext.stmt.break.levels), else one.
    fn levels(&mut self) -> Res<usize> {
        if self.lang.break_levels && self.look().shape == Shape::Numeral {
            let n = self.take().lexeme;
            return n.parse::<usize>().ok().filter(|n| *n >= 1).ok_or_else(|| format!("'{}' is not a number of loops to leave", n));
        }
        Ok(1)
    }

    /// The loop a break or continue of so many levels reaches.
    fn cycle_out(&mut self, levels: usize, what: &str) -> Res<Option<usize>> {
        let unit = self.piece();
        if unit.cycles.is_empty() && unit.outermost {
            return Ok(None);
        }
        if levels > unit.cycles.len() {
            return Err(format!("Cannot '{}' {} level{}", what, levels, if levels == 1 { "" } else { "s" }));
        }
        Ok(Some(unit.cycles.len() - levels))
    }

    fn leave(&mut self, levels: usize) -> Res<()> {
        let owner = self.cycle_out(levels, "break")?;
        let cycle = owner.map(|i| self.piece().cycles[i].began);
        let at = self.departure(cycle);
        match owner {
            Some(i) => self.piece().cycles[i].leaves.push(at),
            None => self.piece().escapes.push(at),
        }
        Ok(())
    }

    fn resume(&mut self, levels: usize) -> Res<()> {
        let owner = self.cycle_out(levels, "continue")?;
        let cycle = owner.map(|i| self.piece().cycles[i].began);
        let at = self.departure(cycle);
        match owner {
            Some(i) => {
                let unit = self.piece();
                match unit.cycles[i].restart {
                    Some(target) => self.patch_jump(at, target),
                    None => unit.cycles[i].resumes.push(at),
                }
            }
            None => self.piece().escapes.push(at),
        }
        Ok(())
    }

    /// A program assembled in its own unit. A function keeps a result
    /// slot: null at first, each expression statement's value after, and
    /// its value is left on the stack at the end.
    fn routine(&mut self, name: &str, formals: Vec<String>, least: usize, returns_value: bool, body: impl FnOnce(&mut Self) -> Res<()>) -> Res<Rc<Routine>> {
        let parameter_rules = self.parameter_rules.take();
        let declared_on = self.declared_at;
        // What the routine around this one carries is put aside while
        // this one is put together, so that each keeps its own.
        let around = std::mem::take(&mut self.carrying);
        // The classes the parameters were declared to take, gathered as
        // they were read. A method is given the object it is for before
        // them, so the list is brought level with the names.
        let mut formal_kinds = std::mem::take(&mut self.formal_kinds);
        while formal_kinds.len() < formals.len() {
            formal_kinds.insert(0, None);
        }
        formal_kinds.truncate(formals.len());
        self.pieces.push(Piece {
            outermost: false,
            ident: name.to_string(),
            idents: formals.clone(),
            declared: vec![false; formals.len()],
            scopes: Vec::new(),
            globals: Vec::new(),
            lasts: Vec::new(),
            cycles: Vec::new(),
            escapes: Vec::new(),
            result_touched: false,
            generator: false,
            line: 0,
            instrs: Vec::new(),
        });
        if returns_value {
            self.constant(Value::Null);
            self.write(RESULT_CELL);
        }
        body(self)?;
        // A function whose value only ever comes from a return
        // drops the result slot: its prologue goes, and a fall off the
        // end leaves nothing, which the machine reads as null.
        let used = self.piece().result_touched;
        if returns_value && used {
            self.read(RESULT_CELL);
        }
        let end = self.mark();
        for at in self.piece().escapes.clone() {
            self.patch_jump(at, end);
        }
        let unit = self.pieces.pop().expect("the unit");
        let mut instrs = if returns_value && !used { relocated(unit.instrs.into_iter().skip(2).collect(), -2) } else { unit.instrs };
        if unit.generator {
            instrs = vec![Instr::Const(Value::text(self.lang.yield_unrun.first().map_or("", String::as_str))),
                Instr::Act(Action::Builtin(Builtin::Raise, Rc::from("")), 1)];
        }
        let within = self.within.as_ref().map(|(named, _)| Rc::from(named.as_str()));
        let carried = std::mem::replace(&mut self.carrying, around);
        Ok(Rc::new(Routine { rest_at: None, ident: unit.ident, formals, parameter_rules, formal_kinds, least, idents: unit.idents, returns_value, body_of_all: false, written_in: self.written_in.clone(), within, declared_on, carried, held: Vec::new(), instrs: Rc::new(peephole(instrs)) }))
    }

    // ---------- statements ----------

    fn stmts_until(&mut self, stops: &[String]) -> Res<()> {
        self.skip_seps();
        while !self.on_any(stops) && !self.exhausted() {
            self.stmt()?;
            self.skip_seps();
        }
        Ok(())
    }

    /// The block after a statement head, in the language's style.
    fn body(&mut self) -> Res<()> {
        if let Some((held, targets)) = self.for_binding.take() {
            let block = self.pos;
            self.pos = targets;
            self.bind_for_targets(&held)?;
            self.pos = block;
        }
        // A loop with nothing to do may be written with the mark that
        // ends a statement standing where its block would: the mark is
        // the whole body, and nothing runs each pass.
        if self.lang.lone_stmt && self.look().shape == Shape::Sign && self.lang.ends_stmt(&self.look().lexeme) {
            self.take();
            return Ok(());
        }
        let introduced = self.on_any(&self.lang.block_intros);
        self.skip_intro();
        if introduced && self.lang.blocks == Blocks::Indented && self.lang.lone_stmt
            && !matches!(self.look().shape, Shape::LineEnd | Shape::Open | Shape::Close | Shape::Finish)
        {
            while !matches!(self.look().shape, Shape::LineEnd | Shape::Close | Shape::Finish) {
                self.stmt()?;
                if self.look().shape == Shape::Sign && self.lang.ends_stmt(&self.look().lexeme) {
                    self.take();
                }
            }
            return Ok(());
        }
        self.skip_seps();
        match self.lang.blocks {
            Blocks::Indented => {
                if self.look().shape != Shape::Open {
                    return Err(format!("Expected an indented block, got '{}'", self.look().lexeme));
                }
                self.take();
                self.skip_seps();
                while self.look().shape != Shape::Close && !self.exhausted() {
                    self.stmt()?;
                    self.skip_seps();
                }
                if self.look().shape != Shape::Close {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.take();
                Ok(())
            }
            Blocks::Braced => {
                let which = self.lang.block_opens.iter().position(|o| self.at_lexeme(o));
                let Some(i) = which else {
                    // A language may open a block with a mark where a
                    // bracket would stand, closing it with a word of its
                    // own. The statements run to whichever word comes
                    // next: one that ends the whole shape is taken here,
                    // and one that opens another arm of it is left
                    // standing for whoever opened the block.
                    if self.lang.instead_mark.as_ref().map_or(false, |m| self.at_symbol(m)) {
                        self.take();
                        let mut stops = self.lang.instead_closes.clone();
                        stops.extend(self.lang.elif_words.iter().cloned());
                        stops.extend(self.lang.else_words.iter().cloned());
                        self.stmts_until(&stops)?;
                        if self.on_any(&self.lang.instead_closes) {
                            self.take();
                        }
                        return Ok(());
                    }
                    if self.lang.lone_stmt {
                        return self.stmt();
                    }
                    return Err(format!("Expected '{}' to open a block, got '{}'", self.lang.block_opens[0], self.look().lexeme));
                };
                self.take();
                let close = self.lang.block_closes[i].clone();
                self.stmts_until(std::slice::from_ref(&close))?;
                self.want_lexeme(&close)
            }
            Blocks::Worded => {
                let closers = self.lang.block_closes.clone();
                self.stmts_until(&closers)?;
                self.expect_closer()
            }
        }
    }

    /// A form may be read whole before the run has the means to keep it.
    fn scope_fault(&mut self, words: &[String]) {
        self.constant(Value::text(words.first().map_or("", String::as_str)));
        self.act(Action::Builtin(Builtin::Raise, Rc::from("")), 1);
    }

    /// A class body has its own reading scope. Its making is still owed.
    fn scoped_class(&mut self) -> Res<()> {
        self.take();
        let named = self.want_name("as the class name")?;
        if self.on_any(&self.lang.class_bases_open) {
            self.take();
            let from = self.mark();
            let close = self.lang.class_bases_close.first().cloned().ok_or("Class bases need a closing mark")?;
            while !self.at_symbol(&close) {
                self.expr(0)?;
                if !self.on_any(&self.lang.tuple_marks) { break; }
                self.take();
            }
            self.want_sign(&close, "after the class bases")?;
            self.piece().instrs.truncate(from);
        }
        let _body = self.routine(&named, Vec::new(), 0, true, |a| a.body())?;
        self.scope_fault(&self.lang.class_unready.clone());
        Ok(())
    }

    /// Commas here join one value, unlike commas between call arguments.
    fn scope_value(&mut self) -> Res<()> {
        let from = self.mark();
        self.expr_at(0, false)?;
        self.scope_tail(from)
    }

    fn scope_tail(&mut self, from: usize) -> Res<()> {
        if !self.on_any(&self.lang.tuple_marks) { return Ok(()); }
        while self.on_any(&self.lang.tuple_marks) {
            self.take();
            if self.on_sep() || matches!(self.look().shape, Shape::Close | Shape::Finish)
                || self.on_assign()
                || self.lang.grouping.as_ref().map_or(false, |g| self.at_symbol(&g.close)) { break; }
            self.expr_at(0, false)?;
        }
        self.piece().instrs.truncate(from);
        self.scope_fault(&self.lang.scope_unready.clone());
        Ok(())
    }

    fn stmt(&mut self) -> Res<()> {
        let lang = self.lang;
        if Lang::spells(&lang.ellipsis_words, &self.look().lexeme)
            && (matches!(self.look_ahead(1).shape, Shape::LineEnd | Shape::Close | Shape::Finish)
                || lang.ends_stmt(&self.look_ahead(1).lexeme)) {
            self.take();
            return Ok(());
        }
        // A language that tells where a complaint happened needs to
        // know which line is running, so each statement says so.
        // Only the program's own lines are marked: what stands before it
        // is the library, and a complaint from inside that names the
        // line of the program that was running, as PHP names it.
        if lang.tells_place && self.look().row as u32 > self.before {
            let row = self.look().row as u32 - self.before;
            if self.piece().line != row {
                self.piece().line = row;
                self.put(Instr::Line(row));
            }
        }
        if self.look().shape == Shape::Instr || self.on_any(&lang.decorator_words) {
            let w = self.look().lexeme.clone();
            if !lang.let_words.is_empty() && Lang::spells(&lang.let_words, &w) {
                return self.binding();
            }
            if !lang.do_words.is_empty() && Lang::spells(&lang.do_words, &w) {
                return self.do_stmt();
            }
            if Lang::spells(&lang.async_words, &w) {
                self.take();
                if !(self.on_keyword(&lang.function_words) || self.on_keyword(&lang.for_words) || self.on_keyword(&lang.with_words)) {
                    return Err(format!("Expected a function, for loop or with block after '{}', got '{}'", w, self.look().lexeme));
                }
                return self.stmt();
            }
            if Lang::spells(&lang.with_words, &w) { return self.with_stmt(); }
            if Lang::spells(&lang.del_words, &w) {
                self.take();
                let call = lang.calling.clone().expect("call marks");
                self.forget_targets(&call, false, true)?;
                self.discard();
                return Ok(());
            }
            if Lang::spells(&lang.nonlocal_words, &w) {
                self.take();
                loop {
                    self.want_name("after the nonlocal keyword")?;
                    if !lang.calling.as_ref().and_then(|c| c.between.as_ref()).map_or(false, |s| self.at_symbol(s)) { break; }
                    self.take();
                }
                self.constant(Value::text(lang.nonlocal_unrun.first().map_or("", String::as_str)));
                self.act(Action::Builtin(Builtin::Raise, Rc::from("")), 1);
                return Ok(());
            }
            if Lang::spells(&lang.if_words, &w) {
                return self.branch();
            }
            if Lang::spells(&lang.while_words, &w) {
                return self.while_stmt();
            }
            if Lang::spells(&lang.until_words, &w) {
                return self.until_stmt();
            }
            if Lang::spells(&lang.for_words, &w) {
                return self.for_stmt();
            }
            if Lang::spells(&lang.return_words, &w) {
                return self.return_stmt();
            }
            if Lang::spells(&lang.break_words, &w) {
                self.take();
                let levels = self.levels()?;
                self.leaving_lasts()?;
                return self.leave(levels);
            }
            if Lang::spells(&lang.continue_words, &w) {
                self.take();
                let levels = self.levels()?;
                self.leaving_lasts()?;
                return self.resume(levels);
            }
            if Lang::spells(&lang.function_words, &w) {
                self.take();
                let gives_cell = self.skip_reference();
                let name = self.want_name("after the function keyword")?;
                return self.function(name, gives_cell);
            }
            if Lang::spells(&lang.pass_words, &w) {
                self.take();
                return Ok(());
            }
            if Lang::spells(&lang.trait_words, &w) {
                return self.bag_decl();
            }
            let names_class = |word: &str| Lang::spells(&lang.class_words, word) || Lang::spells(&lang.interface_words, word);
            if names_class(&w) {
                if !lang.class_bases_open.is_empty() { return self.scoped_class(); }
                return self.class_decl();
            }
            // A class may be marked before it is named: `abstract class C`.
            if Lang::spells(&lang.modifier_words, &w) && self.look_ahead(1).shape == Shape::Instr && names_class(&self.look_ahead(1).lexeme) {
                self.take();
                return self.class_decl();
            }
            if Lang::spells(&lang.try_words, &w) {
                return self.attempt();
            }
            if Lang::spells(&lang.throw_words, &w) {
                self.take();
                if !lang.throw_from.is_empty() && (self.on_sep() || matches!(self.look().shape, Shape::Close | Shape::Finish)) {
                    self.act(Action::Reraise, 0);
                } else {
                    self.expr(0)?;
                    if self.on_keyword(&lang.throw_from) {
                        self.take();
                        let from = self.mark();
                        self.expr(0)?;
                        self.piece().instrs.truncate(from);
                    }
                    self.act(Action::Hurl, 1);
                }
                return Ok(());
            }
            if Lang::spells(&lang.assert_words, &w) {
                self.take();
                self.expr(0)?;
                self.act(Action::Not, 1);
                let passed = self.skip();
                if lang.calling.as_ref().and_then(|b| b.between.as_ref()).map_or(false, |m| self.at_symbol(m)) {
                    self.take();
                    self.expr(0)?;
                } else {
                    self.constant(Value::text(""));
                }
                self.act(Action::AssertFault, 1);
                self.land(passed);
                return Ok(());
            }
            if Lang::spells(&lang.foreach_words, &w) {
                return self.foreach();
            }
            if Lang::spells(&lang.c_for_words, &w) {
                return self.c_for();
            }
            if Lang::spells(&lang.switch_words, &w) {
                return self.switch();
            }
            if Lang::spells(&lang.import_words, &w) || Lang::spells(&lang.import_from_words, &w) {
                return self.import_stmt();
            }
            if Lang::spells(&lang.global_words, &w) {
                return self.global_stmt();
            }
            if Lang::spells(&lang.decorator_words, &w) {
                return self.decorated_function();
            }
            if Lang::spells(&lang.static_words, &w) {
                return self.static_stmt();
            }
            if Lang::spells(&lang.const_words, &w) {
                self.take();
                let name = self.want_name("after the constant keyword")?;
                self.expect_assign("after the constant name")?;
                self.expr(0)?;
                self.write_global(&name);
                return Ok(());
            }
        }
        if lang.blocks == Blocks::Braced && self.on_any(&lang.block_opens) {
            return self.bare_block();
        }
        self.simple_stmt()
    }

    /// Each value is found before its target is filled; the body
    /// follows once all the bindings have been made.
    fn with_stmt(&mut self) -> Res<()> {
        self.take();
        let lang = self.lang;
        let group = lang.grouping.clone().expect("group marks");
        let mut bracketed = false;
        if self.at_symbol(&group.open) {
            let mut depth = 0;
            let mut ahead = 0;
            loop {
                let t = self.look_ahead(ahead);
                if t.shape == Shape::Finish { break; }
                if t.is_lexeme(Shape::Sign, &group.open) { depth += 1; }
                if t.is_lexeme(Shape::Sign, &group.close) {
                    depth -= 1;
                    if depth == 0 {
                        bracketed = Lang::spells(&lang.block_intros, &self.look_ahead(ahead + 1).lexeme);
                        break;
                    }
                }
                ahead += 1;
            }
        }
        if bracketed { self.take(); }
        loop {
            self.expr(0)?;
            if self.on_keyword(&lang.with_as_words) {
                self.take();
                let held = self.gensym("with");
                self.write(&held);
                self.bind_block_target(&held)?;
            } else {
                self.discard();
            }
            if !lang.calling.as_ref().and_then(|c| c.between.as_ref()).map_or(false, |s| self.at_symbol(s)) { break; }
            self.take();
            if bracketed && self.at_symbol(&group.close) { break; }
        }
        if bracketed { self.want_sign(&group.close, "after the with items")?; }
        self.body()
    }

    fn target_count(&self, enclosed: bool) -> (usize, bool, bool) {
        let mut depth = 0;
        let mut count = 0;
        let mut seen = false;
        let mut comma = false;
        let mut starred = false;
        let mut ahead = if enclosed { 1 } else { 0 };
        loop {
            let t = self.look_ahead(ahead);
            if t.shape == Shape::Finish { break; }
            let opening = t.shape == Shape::Sign && (self.lang.grouping.as_ref().map_or(false, |p| t.lexeme == p.open)
                || self.lang.array_brackets.as_ref().map_or(false, |p| t.lexeme == p.open));
            let closing = t.shape == Shape::Sign && (self.lang.grouping.as_ref().map_or(false, |p| t.lexeme == p.close)
                || self.lang.array_brackets.as_ref().map_or(false, |p| t.lexeme == p.close));
            if depth == 0 {
                if (enclosed && closing) || (!enclosed && Lang::spells(&self.lang.in_words, &t.lexeme)) { break; }
                if self.lang.calling.as_ref().and_then(|p| p.between.as_ref()).map_or(false, |s| t.lexeme == *s) {
                    comma = true;
                    seen = false;
                    ahead += 1;
                    continue;
                }
                if !seen { count += 1; seen = true; }
                starred |= self.lang.dyadic.get(&t.lexeme).map_or(false, |op| matches!(op.action, Action::Mul));
            }
            if opening { depth += 1; }
            if closing { depth -= 1; }
            ahead += 1;
        }
        (count, comma, starred)
    }

    fn binding_size(&mut self, held: &str, count: usize, starred: bool) {
        if !starred {
            self.read(held);
            self.act(Action::Builtin(Builtin::Length, Rc::from("")), 1);
            self.constant(Value::Small(count as i64));
            self.act(Action::Ne, 2);
        }
        let skip = if starred { None } else { Some(self.skip()) };
        self.constant(Value::text(&self.lang.binding_unrun));
        self.act(Action::Builtin(Builtin::Raise, Rc::from("")), 1);
        if let Some(skip) = skip { self.land(skip); }
    }

    fn bind_for_targets(&mut self, held: &str) -> Res<()> {
        let (count, comma, starred) = self.target_count(false);
        if !comma { return self.bind_block_target(held); }
        self.binding_size(held, count, starred);
        for index in 0..count {
            let part = self.gensym("item");
            self.read(held);
            self.constant(Value::Small(index as i64));
            self.act(Action::At, 2);
            self.write(&part);
            self.bind_block_target(&part)?;
            if self.lang.calling.as_ref().and_then(|c| c.between.as_ref()).map_or(false, |s| self.at_symbol(s)) { self.take(); }
        }
        Ok(())
    }

    fn bind_block_target(&mut self, held: &str) -> Res<()> {
        if self.lang.dyadic.get(&self.look().lexeme).map_or(false, |op| matches!(op.action, Action::Mul)) {
            self.take();
            self.constant(Value::text(&self.lang.binding_unrun));
            self.act(Action::Builtin(Builtin::Raise, Rc::from("")), 1);
        }
        let group = self.lang.grouping.clone().expect("group marks");
        let array = self.lang.array_brackets.clone().expect("array marks");
        let paired = if self.at_symbol(&group.open) { Some(group) }
            else if self.at_symbol(&array.open) { Some(array) } else { None };
        if let Some(pair) = paired {
            let (count, comma, starred) = self.target_count(true);
            let grouped = !comma && count == 1 && self.lang.grouping.as_ref().map_or(false, |g| g.open == pair.open);
            self.take();
            if grouped {
                self.bind_block_target(held)?;
                self.want_sign(&pair.close, "after the binding target")?;
                return Ok(());
            }
            self.binding_size(held, count, starred);
            let mut index = 0;
            while !self.at_symbol(&pair.close) {
                let part = self.gensym("part");
                self.read(held);
                self.constant(Value::Small(index));
                self.act(Action::At, 2);
                self.write(&part);
                self.bind_block_target(&part)?;
                index += 1;
                if !self.lang.calling.as_ref().and_then(|c| c.between.as_ref()).map_or(false, |s| self.at_symbol(s)) { break; }
                self.take();
                if self.at_symbol(&pair.close) { break; }
            }
            self.want_sign(&pair.close, "after the binding targets")?;
            return Ok(());
        }
        let from = self.mark();
        self.awkward_place = false;
        self.block_place()?;
        if self.awkward_place {
            self.piece().instrs.truncate(from);
            self.constant(Value::text(&self.lang.binding_unrun));
            self.act(Action::Builtin(Builtin::Raise, Rc::from("")), 1);
            return Ok(());
        }
        let waiting = self.waiting.replace(held.to_string());
        let answer = self.store_into(from, None, None, "");
        self.waiting = waiting;
        answer
    }

    /// A place in a binding or deletion; the pipe's mark names a field
    /// here, since no method is being called.
    fn block_place(&mut self) -> Res<()> {
        let name = self.want_name("as a binding target")?;
        self.read(&name);
        self.called_on_value()?;
        let mut keyed = Vec::new();
        loop {
            if self.on_any(&self.lang.pipe_words) {
                self.take();
                keyed.clear();
                let member = self.want_name("after the member mark")?;
                self.act(Action::Grab(Rc::from(member.as_str())), 1);
                self.called_on_value()?;
            } else if let Some(pair) = self.lang.index_brackets.clone().filter(|p| self.at_symbol(&p.open)) {
                self.take();
                let at = self.mark();
                let mut parts = 0;
                let mut sliced = false;
                loop {
                    if self.on_any(&self.lang.block_intros) {
                        self.take(); sliced = true;
                    } else if self.at_symbol(&pair.close) { break; }
                    else if self.lang.calling.as_ref().and_then(|c| c.between.as_ref()).map_or(false, |s| self.at_symbol(s)) {
                        self.take(); sliced = true;
                    } else {
                        if self.on_any(&self.lang.pipe_words) && self.look_ahead(1).lexeme == self.look().lexeme && self.look_ahead(2).lexeme == self.look().lexeme {
                            self.take(); self.take(); self.take();
                            self.constant(Value::Null);
                            sliced = true;
                        } else { self.expr(0)?; }
                        parts += 1;
                        if self.at_symbol(&pair.close) { break; }
                        if !self.on_any(&self.lang.block_intros) && !self.lang.calling.as_ref().and_then(|c| c.between.as_ref()).map_or(false, |s| self.at_symbol(s)) { break; }
                    }
                }
                self.want_sign(&pair.close, "after the index")?;
                if sliced || parts != 1 {
                    self.awkward_place = true;
                    self.piece().instrs.truncate(at);
                    self.constant(Value::Small(0));
                }
                keyed.push(at);
                self.act(Action::At, 2);
            } else { break; }
        }
        self.keyed = keyed;
        Ok(())
    }

    fn simple_stmt(&mut self) -> Res<()> {
        let lang = self.lang;
        if lang.bare_calls && self.look().shape == Shape::Instr {
            // A builtin without brackets after it; echo always, since a
            // bracket after it opens a group, not its arguments.
            let w = self.look().lexeme.clone();
            let bracketed = lang.calling.as_ref().map_or(false, |c| self.look_ahead(1).is_lexeme(Shape::Sign, &c.open));
            match lang.builtins.get(&w) {
                Some(Builtin::Tell) => return self.bare_call(w),
                // A writer written as an operator takes the whole of
                // what follows however it is written, so brackets after
                // it group rather than hold what it is given.
                Some(Builtin::Out) if lang.writes_as_operator => return self.bare_call(w),
                Some(Builtin::Append) | Some(Builtin::Replace) | Some(Builtin::Define) | Some(Builtin::Pack) | Some(Builtin::Erase) | Some(Builtin::Lead) | None => {}
                Some(_) if !bracketed => return self.bare_call(w),
                Some(_) => {}
            }
        }
        self.assign_or_expr()
    }

    /// A store into the global of the name, from anywhere.
    fn write_global(&mut self, name: &str) {
        let slot = Cell { ident: Rc::from(name), near: Vec::new(), far: self.registry.slot(name), moving: false };
        self.put(Instr::Write(slot));
    }

    /// Keep each value before binding the routine, then pass the bound
    /// routine through them from the last written to the first.
    fn decorated_function(&mut self) -> Res<()> {
        let lang = self.lang;
        let amiss = || lang.decorator_amiss.clone().unwrap_or_default();
        let mut held = Vec::new();
        while self.on_any(&lang.decorator_words) {
            self.take();
            self.expr(0)?;
            if self.look().shape != Shape::LineEnd {
                return Err(amiss());
            }
            let name = self.gensym("decorator");
            self.write(&name);
            held.push(name);
            while self.look().shape == Shape::LineEnd {
                self.take();
            }
        }
        if !lang.class_bases_open.is_empty() && self.on_keyword(&lang.class_words) {
            return self.scoped_class();
        }
        if self.on_keyword(&lang.async_words) { self.take(); }
        if !self.on_keyword(&lang.function_words) {
            return Err(amiss());
        }
        self.take();
        let gives_cell = self.skip_reference();
        let name = self.want_name("after the function keyword")?;
        self.function(name.clone(), gives_cell)?;
        for decorator in held.into_iter().rev() {
            let bound = if lang.routines_outermost {
                Cell { ident: Rc::from(name.as_str()), near: Vec::new(), far: self.registry.slot(&name), moving: false }
            } else {
                self.cell_to_write(&name)
            };
            self.put(Instr::Read(bound.clone()));
            self.read_taking(&decorator);
            self.act(Action::Invoke(Rc::from(lang.decorator_words[0].as_str())), 2);
            self.put(Instr::Write(bound));
        }
        Ok(())
    }

    /// A path names what is wanted, without asking any part for a value.
    /// The scanner may already have joined a builtin's dotted spelling.
    fn import_name(&mut self, path: bool) -> Res<String> {
        let word = self.want_name("in an import")?;
        let divider = self.lang.pipe_words.first().map(String::as_str);
        let parts: Vec<&str> = match divider.filter(|_| path) {
            Some(mark) => word.split(mark).collect(),
            None => vec![word.as_str()],
        };
        for part in &parts {
            let mut letters = part.chars();
            if !letters.next().map_or(false, |c| self.lang.begins_name(c))
                || !letters.all(|c| self.lang.extends_name(c)) || self.lang.keywords.contains(*part) {
                return Err(format!("Expected identifier in an import, got '{}'", word));
            }
        }
        let first = parts[0].to_string();
        if path {
            while self.on_any(&self.lang.pipe_words) {
                self.take();
                self.import_name(true)?;
            }
        }
        Ok(first)
    }

    /// Imports give their names places, but no module is carried yet.
    fn import_stmt(&mut self) -> Res<()> {
        let lang = self.lang;
        let from = self.on_keyword(&lang.import_from_words);
        self.take();
        if from {
            let mut relative = false;
            while self.on_any(&lang.pipe_words) || self.on_any(&lang.slice_ellipsis) {
                relative = true;
                self.take();
            }
            if !relative || !self.on_keyword(&lang.import_words) {
                self.import_name(true)?;
            }
            if !self.on_keyword(&lang.import_words) {
                return Err(format!("Expected '{}' after the module name, got '{}'", lang.import_words.first().map_or("", String::as_str), self.look().lexeme));
            }
            self.take();
        }
        let group = lang.grouping.as_ref().filter(|g| from && self.at_symbol(&g.open));
        if group.is_some() {
            self.take();
        }
        let star = from && self.look().shape == Shape::Sign && lang.dyadic.get(&self.look().lexeme).map_or(false, |op| matches!(op.action, Action::Mul));
        if star && group.is_none() {
            self.take();
        } else {
            loop {
                let mut bound = self.import_name(!from)?;
                if self.on_keyword(&lang.import_as_words) {
                    self.take();
                    bound = self.import_name(false)?;
                }
                self.constant(Value::Null);
                self.write(&bound);
                let comma = lang.calling.as_ref().and_then(|g| g.between.as_ref());
                if !comma.map_or(false, |mark| self.at_symbol(mark)) {
                    break;
                }
                self.take();
                if group.map_or(false, |g| self.at_symbol(&g.close)) {
                    break;
                }
            }
        }
        if let Some(g) = group {
            self.want_sign(&g.close, "after the imported names")?;
        }
        if !self.on_sep() && !matches!(self.look().shape, Shape::Close | Shape::Finish) {
            return Err(format!("Unexpected token '{}' after an import", self.look().lexeme));
        }
        Ok(())
    }

    /// `global a, b;`: the names mean the globals in this unit.
    fn global_stmt(&mut self) -> Res<()> {
        self.take();
        let sep = self.lang.calling.as_ref().and_then(|c| c.between.clone());
        loop {
            // A name worked out as the run goes already spells one of
            // the outermost bindings, which is what a global is, so
            // saying so binds nothing further: only the name itself is
            // worked out, and what it spells made ready to be read.
            if self.look().shape != Shape::Instr {
                let seen = self.look().lexeme.clone();
                let from = self.mark();
                self.expr(0)?;
                let read: Vec<Instr> = self.piece().instrs.drain(from..).collect();
                match read.as_slice() {
                    [rest @ .., Instr::Act(Action::Named, 1)] => {
                        let rest = rest.to_vec();
                        let at = self.mark();
                        for w in relocated(rest, at as i64 - from as i64) {
                            self.put(w);
                        }
                        self.act(Action::ReadyNamed, 1);
                        self.discard();
                    }
                    _ => return Err(format!("Expected identifier after the global keyword, got '{}'", seen)),
                }
                match &sep {
                    Some(s) if self.at_symbol(s) => {
                        self.take();
                        continue;
                    }
                    _ => return Ok(()),
                }
            }
            let name = self.want_name("after the global keyword")?;
            // A name bound to a global stands for that global whether or
            // not anything was ever written to it, so the global is made
            // to hold nothing where it held nothing at all: reading it
            // is then reading a name written to.
            let cell = Cell { ident: Rc::from(name.as_str()), near: Vec::new(), far: self.registry.slot(&name), moving: false };
            self.put(Instr::Ready(cell));
            self.piece().globals.push((name.clone(), name));
            match &sep {
                Some(s) if self.at_symbol(s) => {
                    self.take();
                }
                _ => return Ok(()),
            }
        }
    }

    /// `static x = e;`: x names a hidden global, set when the function
    /// is defined, so it keeps its value from call to call. The setting
    /// is assembled in the unit around this one, where the definition
    /// runs. Outside every function there is no unit around this one, so
    /// the setting stands here and is guarded: it happens the first time
    /// this statement is reached and no other time.
    ///
    /// Text read in while the run is already going is read afresh every
    /// time it is reached, so there is no from-call-to-call for such a
    /// statement written at the top of it to keep a value across. There
    /// it is a plain write of the name in the scope that read the text
    /// (ext.stmt.static.read_in), and the scope keeps what was written
    /// as it keeps anything else written to a name of its own.
    fn static_stmt(&mut self) -> Res<()> {
        self.take();
        let sep = self.lang.calling.as_ref().and_then(|c| c.between.clone());
        loop {
            let name = self.want_name("after the static keyword")?;
            // One name kept between calls is one binding: saying so
            // twice in one program is a thing the language refuses.
            if self.piece().globals.iter().any(|(n, _)| *n == name) {
                self.registry.stopped_fatally = true;
                return Err(format!("Duplicate declaration of static variable {}", name));
            }
            if self.read_in && self.lang.static_read_in && self.pieces.len() == 1 {
                // One name is still one binding, though this one binds
                // nothing that would show it: the names spoken for are
                // counted apart so that saying one twice is refused
                // here as it is anywhere else.
                if self.read_statics.iter().any(|n| *n == name) {
                    self.registry.stopped_fatally = true;
                    return Err(format!("Duplicate declaration of static variable {}", name));
                }
                self.read_statics.push(name.clone());
                if self.on_assign() {
                    self.take();
                    self.expr(0)?;
                } else {
                    self.constant(Value::Null);
                }
                self.write(&name);
                match &sep {
                    Some(s) if self.at_symbol(s) => {
                        self.take();
                        continue;
                    }
                    _ => return Ok(()),
                }
            }
            let hidden = self.gensym("static");
            let inner = self.pieces.pop().expect("the unit");
            let outermost = self.pieces.is_empty();
            let mut held = None;
            let mut past = 0;
            if outermost {
                self.pieces.push(inner);
                let far = self.registry.slot(&hidden);
                self.put(Instr::Unwritten(far));
                past = self.skip();
            } else {
                held = Some(inner);
            }
            if self.on_assign() {
                self.take();
                self.expr(0)?;
            } else {
                self.constant(Value::Null);
            }
            self.write_global(&hidden);
            match held {
                Some(inner) => self.pieces.push(inner),
                None => self.land(past),
            }
            self.piece().globals.push((name, hidden));
            match &sep {
                Some(s) if self.at_symbol(s) => {
                    self.take();
                }
                _ => return Ok(()),
            }
        }
    }

    /// Whether the head of a walk, read from where the subject begins,
    /// carries the mark that hands items out for writing. The subject
    /// is stepped over by counting the grouping marks, so that a call
    /// written within it does not look like the end of the head.
    fn hands_over(&mut self, group: &Brackets) -> Res<bool> {
        let lang = self.lang;
        let mark = match &lang.reference_mark {
            Some(mark) => mark.clone(),
            None => return Ok(false),
        };
        let mut at = self.pos;
        let mut deep = 0usize;
        let mut past_as = false;
        while at < self.tokens.len() {
            let w = &self.tokens[at];
            if w.shape == Shape::Sign && w.lexeme == group.open {
                deep += 1;
            } else if w.shape == Shape::Sign && w.lexeme == group.close {
                if deep == 0 {
                    break;
                }
                deep -= 1;
            } else if !past_as {
                if deep == 0 && w.shape == Shape::Instr && Lang::spells(&lang.foreach_as_words, &w.lexeme) {
                    past_as = true;
                }
            } else if w.shape == Shape::Sign && w.lexeme == mark {
                return Ok(true);
            }
            at += 1;
        }
        Ok(false)
    }

    /// `foreach (a as v)` and `foreach (a as k => v)`: the array or map
    /// held aside, walked by position, its key and value bound each pass.
    fn foreach(&mut self) -> Res<()> {
        let lang = self.lang;
        self.take();
        let group = lang.grouping.clone().ok_or_else(|| "A foreach needs syntax.group".to_string())?;
        self.want_sign(&group.open, "after foreach")?;
        // A walk that hands out its items for writing walks the binding
        // itself, so that writing an item writes the array it came from.
        let mut named = match (self.look().shape, self.look_ahead(1).shape) {
            (Shape::Instr, Shape::Instr) if Lang::spells(&lang.foreach_as_words, &self.look_ahead(1).lexeme) => {
                Some(self.take().lexeme)
            }
            _ => None,
        };
        if named.is_none() {
            // What is walked for its own cells need not be written out
            // as a name: a place in an array, a member, or a binding
            // named as the run goes has a cell as well. Where the head
            // hands items out, such a subject is asked for its cell and
            // a hidden name fastened to it, which the walk then treats
            // as the name it was not given.
            if self.hands_over(&group)? {
                let held = self.gensym("walked");
                self.a_cell(&self.lang.unshared_written.clone(), false, None)?;
                let cell = self.cell_to_write(&held);
                self.put(Instr::Fasten(cell));
                named = Some(held);
            } else {
                self.expr(0)?;
            }
        }
        if !self.on_keyword(&lang.foreach_as_words) {
            return Err(format!("Expected '{}' in foreach, got '{}'", lang.foreach_as_words[0], self.look().lexeme));
        }
        self.take();
        let mut shared = self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m));
        // The sign that hands items out may stand before the value
        // rather than before the pair, so the whole of the head is read
        // through before the walk's subject is settled: a walk that
        // hands items out walks the binding itself, and one that does
        // not walks what the binding held when the walk began, so that
        // writing to it while it is walked changes nothing.
        let hands_out = match &lang.reference_mark {
            None => false,
            Some(mark) => {
                let mut at = self.pos;
                let mut found = false;
                while at < self.tokens.len() {
                    let w = &self.tokens[at];
                    if w.shape == Shape::Sign && w.lexeme == group.close {
                        break;
                    }
                    if w.shape == Shape::Sign && w.lexeme == *mark {
                        found = true;
                        break;
                    }
                    at += 1;
                }
                found
            }
        };
        let bag = match &named {
            Some(name) if hands_out => name.clone(),
            _ => {
                let bag = self.gensym("bag");
                if let Some(name) = &named {
                    let name = name.clone();
                    self.read(&name);
                }
                self.write(&bag);
                bag
            }
        };
        // Whether the mark stood before the whole of the head rather
        // than before the value alone, which tells a key marked as
        // taking a cell from a value so marked.
        let marked_first = shared;
        if shared {
            self.take();
        }
        // A walk may hand its items to a place and not only to a name:
        // `foreach ($a as $b[0])`. Where the name is followed by more,
        // where it began is kept and read again at the top of each
        // pass, the item waiting in a cell of the walk's own.
        let first_at = self.pos;
        let first = self.want_name("as the foreach variable")?;
        let paired = lang.pair_mark.as_ref().map_or(false, |m| self.at_symbol(m));
        let mut place: Option<usize> = None;
        let (key, mut value) = if paired {
            // A key is not a place: it is what a member is called, and a
            // name given it has no cell of the walk's to be fastened to.
            if let (true, Some(said)) = (marked_first, &lang.walk_key_no_cell) {
                self.registry.stopped_fatally = true;
                return Err(said.clone());
            }
            self.take();
            if self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) {
                self.take();
                shared = true;
            }
            let began = self.pos;
            let held = self.want_name("as the foreach value")?;
            if !self.at_symbol(&group.close) {
                place = Some(began);
            }
            (Some(first), held)
        } else {
            if !self.at_symbol(&group.close) {
                place = Some(first_at);
            }
            (None, first)
        };
        if place.is_some() {
            if shared {
                return Err("A walk hands its items for writing to a name, not to a place".to_string());
            }
            self.skip_to_close(&group)?;
            value = self.gensym("item");
        }
        if shared && named.is_none() {
            return Err("A foreach that hands out its items for writing needs a named array".to_string());
        }
        self.want_sign(&group.close, "after the foreach names")?;
        self.walk(&bag, key.as_deref(), &value, shared, place)
    }

    /// Step to the mark that closes a grouping, reading nothing on the
    /// way, so that what stands within it may be read later.
    fn skip_to_close(&mut self, group: &Brackets) -> Res<()> {
        let mut deep = 1usize;
        while deep > 0 {
            if self.exhausted() {
                return Err(format!("Expected '{}'", group.close));
            }
            if self.at_symbol(&group.open) {
                deep += 1;
            } else if self.at_symbol(&group.close) {
                deep -= 1;
                if deep == 0 {
                    break;
                }
            }
            self.take();
        }
        Ok(())
    }

    /// The walk itself: a place counted up to the extent, the key and
    /// the value bound from it at the head of each pass.
    fn walk(&mut self, bag: &str, key: Option<&str>, value: &str, shared: bool, place: Option<usize>) -> Res<()> {
        let lang = self.lang;
        // A walk that hands out the items' own cells goes over the array
        // as it stands: what the body does to the array it does to the
        // walk. One that walks a copy takes the copy here, and what the
        // body does to the array is nothing to it — save where what is
        // walked is a thing, which is a handle and so is walked itself.
        // A thing that is its own walk, or that hands another over to be
        // walked in its stead, is also settled here, once.
        if shared {
            // A thing that is its own walk holds no cells of its own to
            // hand out, and a language with words for that says so.
            self.read(bag);
            self.act(Action::WalkAlone, 1);
            self.put_away();
        }
        let over = match shared {
            true => bag.to_string(),
            false => {
                let copy = self.gensym("walk");
                self.read(bag);
                self.act(Action::WalkFrom, 1);
                self.write(&copy);
                copy
            }
        };
        let at = self.gensym("at");
        self.constant(Value::Small(0));
        self.write(&at);
        // Where the language keeps a walk's place by the item it handed
        // out, the cell of that item is kept here too, so that the pass
        // after can ask where the item has got to. It holds nothing
        // until the first item is handed out.
        let held = (shared && lang.walk_alive).then(|| self.gensym("held"));
        if let Some(held) = &held {
            self.constant(Value::Null);
            self.write(held);
        }
        let to_test = self.leap();
        let top = self.mark();
        self.enter_cycle(None);
        // A member taken off a thing while the walk is under way leaves
        // its place behind, so that the members after it keep the places
        // they had. The walk steps over such a place, as though the body
        // of that pass had gone straight on to the next. Only a language
        // with things to take members off asks the question at all.
        if !lang.class_words.is_empty() {
            // A walk hands out only what it may reach: the class it is
            // written in, if it is written in one, settles that.
            let here = self.within.as_ref().map(|(named, _)| Rc::from(named.as_str()));
            self.read(&over);
            self.read(&at);
            self.act(Action::Standing(here), 2);
            let step_over = self.skip();
            let deep = self.piece().cycles.len() - 1;
            self.piece().cycles[deep].resumes.push(step_over);
        }
        // What stands here is asked for before what it is called, since
        // a thing that is its own walk is asked both and answers in that
        // order.
        if shared {
            // The name is fastened to the item's own cell.
            self.read(&at);
            let from = self.cell_to_read(bag, false);
            self.put(Instr::BondItem(from));
            let name = self.cell_to_write(value);
            self.put(Instr::Fasten(name));
            // The walk's own name is fastened to it as well: asking the
            // place for its cell a second time answers with the very
            // same one, since a place already shared is shared already.
            if let Some(held) = &held {
                self.read(&at);
                let again = self.cell_to_read(bag, false);
                self.put(Instr::BondItem(again));
                let keep = self.cell_to_write(held);
                self.put(Instr::Fasten(keep));
            }
        } else {
            self.read(&over);
            self.read(&at);
            self.act(Action::WalkThis, 2);
            self.write(value);
        }
        if let Some(key) = key {
            self.read(&over);
            self.read(&at);
            self.act(Action::WalkKey, 2);
            self.write(key);
        }
        // Where the walk hands its items to a place, the place is read
        // again here, with the item waiting in the walk's own cell.
        if let Some(began) = place {
            let after = self.pos;
            self.pos = began;
            let from = self.mark();
            self.expr_at(0, false)?;
            let was = self.waiting.replace(value.to_string());
            let done = self.store_into(from, None, None, "=");
            self.waiting = was;
            self.pos = after;
            done?;
        }
        self.body()?;
        let again = self.mark();
        match &held {
            // The place the walk goes on from is the one past where the
            // item it handed out now lies, which is not where it lay
            // before if the body has moved it along or shortened the
            // array in front of it.
            Some(held) => {
                self.read(&over);
                self.read(&at);
                let keep = self.cell_to_write(held);
                self.put(Instr::Bond(keep));
                self.act(Action::WalkPast, 3);
                self.write(&at);
            }
            None => {
                self.read(&at);
                self.constant(Value::Small(1));
                self.act(Action::Add, 2);
                self.write(&at);
            }
        }
        self.read(&over);
        self.act(Action::WalkOnward, 1);
        self.put_away();
        self.land(to_test);
        self.read(&over);
        self.read(&at);
        self.act(Action::WalkMore, 2);
        self.loop_back(top);
        self.complete_cycle(again)?;
        // The walk lets its last item go once it is over, so that the
        // place holding it is a place two names share only while some
        // name of the program's own still holds it.
        if let Some(held) = &held {
            let keep = self.cell_to_write(held);
            self.put(Instr::Forget(keep));
        }
        Ok(())
    }

    /// `for (init; test; step) body`, the test at the bottom as in a
    /// while loop; `continue` goes to the step. An empty test is true.
    fn c_for(&mut self) -> Res<()> {
        let lang = self.lang;
        self.take();
        let group = lang.grouping.clone().ok_or_else(|| "A for loop needs syntax.group".to_string())?;
        let sep = lang.calling.as_ref().and_then(|c| c.between.clone());
        let end = |a: &Self| a.look().shape == Shape::Sign && lang.ends_stmt(&a.look().lexeme);
        self.want_sign(&group.open, "after for")?;
        // init
        while !end(self) {
            self.simple_stmt()?;
            match &sep {
                Some(s) if self.at_symbol(s) => {
                    self.take();
                }
                _ => break,
            }
        }
        self.take();
        // The test is read once to find its end, then again after the body.
        let cond_at = self.pos;
        let has_test = !end(self);
        let mark = self.mark();
        if has_test {
            self.expr(0)?;
        }
        self.piece().instrs.truncate(mark);
        self.take();
        // The step, likewise, is read once to find the body.
        let step_at = self.pos;
        let mark = self.mark();
        while !self.at_symbol(&group.close) && !self.exhausted() {
            self.simple_stmt()?;
            match &sep {
                Some(s) if self.at_symbol(s) => {
                    self.take();
                }
                _ => break,
            }
        }
        self.piece().instrs.truncate(mark);
        self.want_sign(&group.close, "after the for clauses")?;
        let to_test = self.leap();
        let top = self.mark();
        self.enter_cycle(None);
        self.body()?;
        let after = self.pos;
        let again = self.mark();
        self.pos = step_at;
        while !self.at_symbol(&group.close) && !self.exhausted() {
            self.simple_stmt()?;
            match &sep {
                Some(s) if self.at_symbol(s) => {
                    self.take();
                }
                _ => break,
            }
        }
        self.land(to_test);
        if has_test {
            self.pos = cond_at;
            self.expr(0)?;
            self.loop_back(top);
        } else {
            self.constant(Value::Flag(false));
            self.put(Instr::Skip(top));
        }
        self.pos = after;
        self.leave_cycle(again);
        Ok(())
    }

    /// `switch (v) { case a: ... default: ... }`: the value kept in a
    /// hidden slot, each case a test that skips to the next test when it
    /// fails; a body runs on into the next (over its test) unless it
    /// breaks. When every test fails the default's body runs, wherever
    /// it stands. A switch is a loop to `break` and `continue`.
    fn switch(&mut self) -> Res<()> {
        let lang = self.lang;
        self.take();
        let group = lang.grouping.clone().ok_or_else(|| "A switch needs syntax.group".to_string())?;
        self.want_sign(&group.open, "after switch")?;
        self.expr(0)?;
        self.want_sign(&group.close, "after the switch value")?;
        let subject = self.gensym("switch");
        self.write(&subject);
        self.skip_intro();
        self.skip_seps();
        // A switch is opened by a bracket or, where the language spells
        // one, by the mark that stands for a bracket; the words closing
        // such a block close this one.
        let closers = match lang.block_opens.iter().position(|o| self.at_lexeme(o)) {
            Some(i) => {
                self.take();
                vec![lang.block_closes[i].clone()]
            }
            None if lang.instead_mark.as_ref().map_or(false, |m| self.at_symbol(m)) => {
                self.take();
                lang.instead_closes.clone()
            }
            None => return Err(format!("Expected '{}' to open the switch, got '{}'", lang.block_opens[0], self.look().lexeme)),
        };
        let mut stops = closers.clone();
        stops.extend(lang.case_words.iter().cloned());
        stops.extend(lang.default_words.iter().cloned());
        self.enter_cycle(None);
        // A failed test waits for the next test; a body's end waits for
        // the next body.
        let mut failed: Option<usize> = None;
        let mut fell: Option<usize> = None;
        let mut default_at: Option<usize> = None;
        self.skip_seps();
        while !self.on_any(&closers) && !self.exhausted() {
            let word = self.want_name("as case or default")?;
            let is_case = Lang::spells(&lang.case_words, &word);
            if !is_case && !Lang::spells(&lang.default_words, &word) {
                return Err(format!("Expected a case in the switch, got '{}'", word));
            }
            if is_case {
                if let Some(at) = failed.take() {
                    self.land(at);
                }
                self.read(&subject);
                self.expr(0)?;
                self.act(Action::Eq, 2);
                failed = Some(self.skip());
            }
            if !(self.on_any(&lang.case_marks) || self.on_sep()) {
                return Err(format!("Expected '{}' after the case, got '{}'", lang.case_marks[0], self.look().lexeme));
            }
            // A case closed the way a statement is closed is allowed and
            // asked to be written the other way, where the language has
            // words for it.
            if let (false, Some(said)) = (self.on_any(&lang.case_marks), &lang.case_mark_instead) {
                let row = (self.look().row as u32).saturating_sub(self.before);
                let words = (Complaint::Deprecated, said.clone(), row);
                self.registry.said_while_reading.push(words);
            }
            self.take();
            if let Some(at) = fell.take() {
                self.land(at);
            }
            if !is_case {
                default_at = Some(self.mark());
            }
            self.stmts_until(&stops)?;
            fell = Some(self.leap());
        }
        if !self.on_any(&closers) {
            return Err(format!("Expected '{}' to close the switch, got '{}'", closers[0], self.look().lexeme));
        }
        self.take();
        let end = self.mark();
        if let Some(at) = fell {
            self.land(at);
        }
        if let Some(at) = failed {
            match default_at {
                Some(body) => self.piece().instrs[at] = Instr::Skip(body),
                None => self.land(at),
            }
        }
        self.leave_cycle(end);
        Ok(())
    }

    /// The increment or decrement a sign spells, if any.
    fn bump_of(&self, tok: &Token) -> Option<Action> {
        if tok.shape != Shape::Sign {
            return None;
        }
        if self.lang.increments.iter().any(|s| *s == tok.lexeme) {
            Some(Action::Step(true))
        } else if self.lang.decrements.iter().any(|s| *s == tok.lexeme) {
            Some(Action::Step(false))
        } else {
            None
        }
    }

    /// A builtin at the head of a statement called without brackets:
    /// its arguments, separated like call arguments, run to the end of
    /// the statement (ext.syntax.call.bare).
    fn bare_call(&mut self, name: String) -> Res<()> {
        self.take();
        let sep = self.lang.calling.as_ref().and_then(|c| c.between.clone());
        let mut argc = 0;
        while !self.on_sep() && !self.exhausted() {
            self.expr(0)?;
            argc += 1;
            match &sep {
                Some(s) if self.at_symbol(s) => {
                    self.take();
                }
                _ => break,
            }
        }
        self.call(&name, argc)?;
        self.piece().result_touched = true;
        self.write(RESULT_CELL);
        Ok(())
    }

    /// A bare block's bindings are forgotten on leaving it.
    fn bare_block(&mut self) -> Res<()> {
        self.piece().scopes.push(Vec::new());
        self.body()?;
        let bound = self.piece().scopes.pop().expect("the block");
        for s in bound {
            let name = self.piece().idents[s].clone();
            let global = self.registry.slot(&name);
            let slot = Cell { ident: Rc::from(name.as_str()), near: vec![s], far: global, moving: false };
            self.constant(Value::Blank);
            self.put(Instr::Write(slot));
        }
        Ok(())
    }

    fn binding(&mut self) -> Res<()> {
        let lang = self.lang;
        if lang.types_first {
            return self.typed_declaration();
        }
        self.take();
        if self.on_keyword(&lang.mutable_words) {
            self.take();
        }
        let name = self.want_name("after the binding keyword")?;
        if self.look().shape == Shape::Sign && Lang::spells(&lang.type_marks, &self.look().lexeme) {
            self.take();
            self.want_name("as a type name")?;
        }
        if self.on_sep() || self.exhausted() {
            self.constant(Value::Null);
        } else {
            self.expect_assign("in a binding")?;
            self.expr(0)?;
        }
        self.write(&name);
        Ok(())
    }

    /// C: the type word leads; a call bracket after the name is a function.
    fn typed_declaration(&mut self) -> Res<()> {
        self.take();
        let name = self.want_name("after the type")?;
        if let Some(call) = &self.lang.calling {
            if self.at_symbol(&call.open) {
                return self.function(name, false);
            }
        }
        if self.on_sep() || self.exhausted() {
            self.constant(Value::Null);
        } else {
            self.expect_assign("in a declaration")?;
            self.expr(0)?;
        }
        self.write(&name);
        Ok(())
    }

    fn on_assign(&self) -> bool {
        self.look().shape == Shape::Sign && Lang::spells(&self.lang.assign_words, &self.look().lexeme)
    }

    fn expect_assign(&mut self, why: &str) -> Res<()> {
        if self.lang.assign_words.is_empty() {
            return Err("This language has no assignment operator".to_string());
        }
        if !self.on_assign() {
            return Err(format!("Expected '{}' {}, got '{}'", self.lang.assign_words[0], why, self.look().lexeme));
        }
        self.take();
        Ok(())
    }

    /// `if c block [elif c block]* [else block]`; one closer ends a
    /// keyword-style chain.
    fn branch(&mut self) -> Res<()> {
        let lang = self.lang;
        let keyword_style = lang.blocks == Blocks::Worded;
        self.take();
        self.expr(0)?;
        let skip = self.skip();
        if keyword_style {
            self.skip_intro();
            let mut stops = lang.block_closes.clone();
            stops.extend(lang.elif_words.iter().cloned());
            stops.extend(lang.else_words.iter().cloned());
            self.stmts_until(&stops)?;
        } else {
            self.body()?;
        }
        // An else may come after line ends.
        let mut ahead = 0;
        loop {
            let t = self.look_ahead(ahead);
            if t.shape == Shape::LineEnd || (t.shape == Shape::Sign && lang.ends_stmt(&t.lexeme)) {
                ahead += 1;
            } else {
                break;
            }
        }
        let t = self.look_ahead(ahead);
        let elif = t.shape == Shape::Instr && Lang::spells(&lang.elif_words, &t.lexeme);
        let els = t.shape == Shape::Instr && Lang::spells(&lang.else_words, &t.lexeme);
        if !elif && !els {
            if keyword_style {
                self.expect_closer()?;
            }
            self.land(skip);
            return Ok(());
        }
        let over = self.leap();
        self.land(skip);
        self.pos += ahead;
        if elif {
            self.branch()?;
        } else {
            self.take();
            if self.on_keyword(&lang.if_words) {
                self.branch()?;
            } else if keyword_style {
                let closers = lang.block_closes.clone();
                self.stmts_until(&closers)?;
                self.expect_closer()?;
            } else {
                self.body()?;
            }
        }
        self.land(over);
        Ok(())
    }

    /// `do body while (c);`: the body runs before the test is asked, so
    /// it runs at least once. A continue goes to the test, as it goes to
    /// the step of a counted loop.
    fn do_stmt(&mut self) -> Res<()> {
        let lang = self.lang;
        self.take();
        let top = self.mark();
        self.enter_cycle(None);
        self.body()?;
        let again = self.mark();
        if !self.on_keyword(&lang.while_words) {
            return Err(format!("Expected '{}' after the body, got '{}'", lang.while_words.first().map_or("while", |w| w.as_str()), self.look().lexeme));
        }
        self.take();
        let group = lang.grouping.clone().ok_or_else(|| "A do loop needs syntax.group".to_string())?;
        self.want_sign(&group.open, "after while")?;
        self.expr(0)?;
        self.want_sign(&group.close, "after the condition")?;
        self.loop_back(top);
        self.leave_cycle(again);
        Ok(())
    }

    /// `while c body`, tested at the bottom: one jump per pass instead of
    /// two. The condition is read once to find the body, discarded, and
    /// read again after it.
    fn while_stmt(&mut self) -> Res<()> {
        self.take();
        let cond_at = self.pos;
        // Skip the condition's tokens for now: parse it once, discard.
        let mark = self.mark();
        self.expr(0)?;
        let body_at = self.pos;
        self.piece().instrs.truncate(mark);
        let to_test = self.leap();
        let top = self.mark();
        self.enter_cycle(None);
        self.pos = body_at;
        self.body()?;
        let after = self.pos;
        let test = self.mark();
        self.land(to_test);
        self.pos = cond_at;
        self.expr(0)?;
        self.pos = after;
        self.loop_back(top);
        self.complete_cycle(test)?;
        Ok(())
    }

    /// `until c block`: the body first, then stop once c holds. The
    /// condition is written first but tested after the body, so its instrs
    /// are lifted out and put back after it.
    fn until_stmt(&mut self) -> Res<()> {
        self.take();
        let from = self.mark();
        self.expr(0)?;
        let test: Vec<Instr> = self.piece().instrs.drain(from..).collect();
        let top = self.mark();
        self.enter_cycle(None);
        self.body()?;
        let again = self.mark();
        for w in relocated(test, again as i64 - from as i64) {
            self.put(w);
        }
        self.put(Instr::Skip(top));
        self.leave_cycle(again);
        Ok(())
    }

    /// `for v in a..b block`: a counted loop with the bound in a hidden slot.
    fn for_stmt(&mut self) -> Res<()> {
        let lang = self.lang;
        self.take();
        let var = if lang.loop_else && !Lang::spells(&lang.in_words, &self.look_ahead(1).lexeme) {
            let held = self.gensym("iteration");
            let targets = self.pos;
            let from = self.mark();
            self.bind_for_targets(&held)?;
            self.piece().instrs.truncate(from);
            self.for_binding = Some((held.clone(), targets));
            held
        } else { self.want_name("as the loop variable")? };
        if !self.on_keyword(&lang.in_words) {
            return Err(format!("Expected '{}' after for loop variable, got: {}", lang.in_words[0], self.look().lexeme));
        }
        self.take();
        // A range value uses the builtin's arity and step, then is walked.
        let range_call = !lang.range_value && self.look().shape == Shape::Instr
            && lang.builtins.get(&self.look().lexeme) == Some(&Builtin::Span)
            && lang.calling.as_ref().map_or(false, |c| self.look_ahead(1).is_lexeme(Shape::Sign, &c.open));
        if range_call {
            self.take();
            let call = lang.calling.clone().expect("call brackets");
            self.take();
            self.expr(0)?;
            if lang.loop_else && self.at_symbol(&call.close) {
                let bound = self.gensym("end");
                self.write(&bound);
                self.constant(Value::Small(0));
                self.write(&var);
                self.take();
                return self.count_loop(&var, &bound, false);
            }
            self.write(&var);
            if let Some(sep) = &call.between {
                self.want_sign(sep, "between the range bounds")?;
            }
            self.expr(0)?;
            self.want_sign(&call.close, "after the range")?;
        } else {
            let tier = lang.range_marks.iter().filter_map(|r| lang.precedence.get(r)).min().copied().unwrap_or(0);
            let from = self.mark();
            self.expr(tier + 1)?;
            if !(self.look().shape == Shape::Sign && Lang::spells(&lang.range_marks, &self.look().lexeme)) {
                // Not a range: what was read is a thing to walk through.
                if !lang.for_collections {
                    return Err("A for loop needs a range: start..end".to_string());
                }
                let bag = self.gensym("bag");
                let source: Vec<Instr> = self.piece().instrs.drain(from..).collect();
                for w in relocated(source, -(from as i64) + self.mark() as i64) {
                    self.put(w);
                }
                self.write(&bag);
                return self.walk(&bag, None, &var, false, None);
            }
            self.take();
            self.write(&var);
            self.expr(tier + 1)?;
        }
        let bound = self.gensym("end");
        self.write(&bound);
        self.count_loop(&var, &bound, false)
    }

    /// The loop itself, the variable holding the start and the bound stored.
    fn count_loop(&mut self, var: &str, bound: &str, postfix: bool) -> Res<()> {
        let to_test = self.leap();
        let top = self.mark();
        self.enter_cycle(None);
        if postfix {
            self.rpn_block()?;
        } else {
            self.body()?;
        }
        let again = self.mark();
        self.read(var);
        self.constant(Value::Small(1));
        self.act(Action::Add, 2);
        self.write(var);
        self.land(to_test);
        self.read(var);
        self.read(bound);
        self.act(Action::Lt, 2);
        self.loop_back(top);
        self.complete_cycle(again)?;
        Ok(())
    }

    fn return_stmt(&mut self) -> Res<()> {
        self.take();
        // A routine that gives back a cell answers with the cell of
        // whatever it names, so a name fastened to the answer and the
        // one inside the routine stand for the one cell. Where what it
        // names has no cell to share, the value itself is answered, as
        // such a language does rather than stopping.
        let by_cell = self.giving_cells.last().copied().unwrap_or(false);
        if self.on_sep() || self.exhausted() || self.on_any(&self.lang.block_closes) {
            self.constant(Value::Null);
        } else if by_cell {
            self.a_cell(&self.lang.unshared_given.clone(), true, None)?;
        } else {
            if self.lang.tuple_marks.is_empty() { self.expr(0)?; } else { self.scope_value()?; }
        }
        // The value is worked out first, then the last parts of any open
        // try statements run, and only then does the program leave.
        let value = self.gensym("returned");
        let held = !self.piece().lasts.is_empty();
        if held {
            self.write(&value);
        }
        self.leaving_lasts()?;
        if held {
            self.read(&value);
        }
        self.escape();
        Ok(())
    }

    /// From the parameter list on: `( formals ) [returns type] block`,
    /// with declarations before the body where the language has them.
    /// `try { } catch (A | B $e) { } finally { }`: the body runs under a
    /// guard, and a value raised inside it arrives on the stack where the
    /// guard points. Each clause says which classes it takes; what no
    /// clause takes is raised again. The last part, if there is one, runs
    /// on both ways out, so it is written twice.
    fn attempt(&mut self) -> Res<()> {
        if !self.lang.catch_as.is_empty() {
            return self.indented_attempt();
        }
        let lang = self.lang;
        self.take();
        // Where the last part begins, found before the body is read, so
        // that a return inside the body can run it first.
        let last = self.last_part_ahead();
        let guard = self.put(Instr::Guard(0));
        if let Some(at) = last {
            self.piece().lasts.push(at);
        }
        self.body()?;
        self.put(Instr::Unguard);
        let mut done = vec![self.leap()];
        let here = self.mark();
        self.piece().instrs[guard] = Instr::Guard(here);
        let group = lang.grouping.clone().ok_or_else(|| "A catch needs syntax.group".to_string())?;
        let mut clauses = 0;
        // A clause may stand on a line of its own, after the body it follows.
        self.skip_seps();
        while self.on_keyword(&lang.catch_words) {
            self.take();
            self.want_sign(&group.open, "after catch")?;
            let mut classes = vec![self.want_name("as the class caught")?];
            while lang.catch_between.as_ref().map_or(false, |s| self.at_symbol(s)) {
                self.take();
                classes.push(self.want_name("as another class caught")?);
            }
            let held = match self.look().shape {
                Shape::Instr => Some(self.take().lexeme),
                _ => None,
            };
            self.want_sign(&group.close, "after the class caught")?;
            self.act(Action::Twin, 1);
            self.act(Action::Matches(Rc::new(classes)), 1);
            let past = self.skip();
            match held {
                Some(name) => self.write(&name),
                None => self.discard(),
            }
            self.body()?;
            done.push(self.leap());
            self.land(past);
            clauses += 1;
            self.skip_seps();
        }
        if last.is_some() {
            self.piece().lasts.pop();
        }
        if clauses == 0 && last.is_none() {
            return Err("A try needs a catch or a last part".to_string());
        }
        self.last_part(last)?;
        self.act(Action::Hurl, 1);
        for at in done {
            self.land(at);
        }
        self.last_part(last)?;
        Ok(())
    }

    /// A colon body may stand on the same line as its head. This small
    /// reading belongs to the watched statement and each of its arms.
    fn attempt_body(&mut self) -> Res<(usize, usize)> {
        let start = self.mark();
        if self.lang.blocks == Blocks::Indented && self.on_any(&self.lang.block_intros) {
            self.take();
            if self.look().shape != Shape::LineEnd {
                loop {
                    self.stmt()?;
                    if self.look().shape != Shape::Sign || !self.lang.ends_stmt(&self.look().lexeme) { break; }
                    self.take();
                    if matches!(self.look().shape, Shape::LineEnd | Shape::Finish) { break; }
                }
                return Ok((start, self.mark()));
            }
        }
        self.body()?;
        Ok((start, self.mark()))
    }

    fn indented_attempt(&mut self) -> Res<()> {
        self.take();
        let mark = self.put(Instr::Attempt(Box::new(Attempt {
            body: (0, 0), clauses: Vec::new(), otherwise: None, last: None, after: 0,
        })));
        let body = self.attempt_body()?;
        let mut clauses = Vec::new();
        self.skip_seps();
        let lang = self.lang;
        while self.on_keyword(&lang.catch_words) {
            self.take();
            let grouped = lang.catch_group.as_ref().map_or(false, |m| self.at_symbol(m));
            if grouped { self.take(); }
            let tuple = lang.catch_tuple_open.as_ref().map_or(false, |m| self.at_symbol(m));
            if tuple { self.take(); }
            let mut kinds = Vec::new();
            let bare = !tuple && self.on_any(&lang.block_intros);
            let empty = tuple && lang.catch_tuple_close.as_ref().map_or(false, |m| self.at_symbol(m));
            if !bare && !empty {
                loop {
                    let from = self.mark();
                    self.expr(0)?;
                    let to = self.mark();
                    // A lone missing name takes nothing, without a complaint.
                    if to == from + 1 {
                        if let Instr::Read(cell) = self.piece().instrs[from].clone() {
                            self.piece().instrs[from] = Instr::Glance(cell);
                        }
                    }
                    kinds.push((from, to));
                    if !lang.catch_between.as_ref().map_or(false, |m| self.at_symbol(m)) { break; }
                    self.take();
                    if tuple && lang.catch_tuple_close.as_ref().map_or(false, |m| self.at_symbol(m)) { break; }
                }
            }
            if tuple {
                self.want_sign(lang.catch_tuple_close.as_deref().unwrap_or(")"), "after the classes caught")?;
            }
            let held = if self.on_keyword(&lang.catch_as) {
                self.take();
                let name = self.want_name("after the caught value's binding word")?;
                Some(self.cell_to_write(&name))
            } else { None };
            let arm = self.attempt_body()?;
            clauses.push(Taking { kinds, held, body: arm, grouped, bare });
            self.skip_seps();
        }
        let otherwise = if lang.try_else && self.on_keyword(&lang.else_words) {
            self.take();
            Some(self.attempt_body()?)
        } else { None };
        self.skip_seps();
        let last = if self.on_keyword(&lang.finally_words) {
            self.take();
            Some(self.attempt_body()?)
        } else { None };
        if clauses.is_empty() && (last.is_none() || otherwise.is_some()) {
            return Err("A try needs a catch or a last part".to_string());
        }
        let after = self.mark();
        self.piece().instrs[mark] = Instr::Attempt(Box::new(Attempt { body, clauses, otherwise, last, after }));
        Ok(())
    }

    /// The last part of a try statement, read again from where it stands.
    fn last_part(&mut self, at: Option<usize>) -> Res<()> {
        let Some(at) = at else { return Ok(()) };
        self.pos = at;
        self.take();
        self.body()
    }

    /// Every open try statement's last part, innermost first: what a
    /// return, a break or a continue runs before it leaves them. Where
    /// the reader stands is put back afterwards.
    fn leaving_lasts(&mut self) -> Res<()> {
        let held = self.pos;
        for at in self.piece().lasts.clone().into_iter().rev() {
            self.last_part(Some(at))?;
        }
        self.pos = held;
        Ok(())
    }

    /// Where the last part of this try statement begins, if it has one:
    /// the token after the body and any catch clauses.
    fn last_part_ahead(&self) -> Option<usize> {
        let lang = self.lang;
        let (mut depth, mut i) = (0usize, self.pos);
        while i < self.tokens.len() && self.tokens[i].shape != Shape::Finish {
            let t = &self.tokens[i];
            let lexeme_here = |list: &[String]| matches!(t.shape, Shape::Sign | Shape::Instr) && list.iter().any(|x| *x == t.lexeme);
            if lexeme_here(&lang.block_opens) {
                depth += 1;
            } else if lexeme_here(&lang.block_closes) {
                depth -= 1;
                if depth == 0 {
                    let mut j = i + 1;
                    while j < self.tokens.len() {
                        let w = &self.tokens[j];
                        let sep = w.shape == Shape::LineEnd || (w.shape == Shape::Sign && lang.ends_stmt(&w.lexeme));
                        if !sep {
                            break;
                        }
                        j += 1;
                    }
                    let w = &self.tokens[j];
                    if w.shape != Shape::Instr {
                        return None;
                    }
                    if Lang::spells(&lang.finally_words, &w.lexeme) {
                        return Some(j);
                    }
                    if !Lang::spells(&lang.catch_words, &w.lexeme) {
                        return None;
                    }
                    i = j;
                }
            }
            i += 1;
        }
        None
    }

    /// A bag of members a class may take in as its own. Nothing of it
    /// runs and nothing is bound to its name: where its body begins is
    /// written down, and every class taking it in reads that body again
    /// as its own, so the members stand in the class and not in the bag.
    fn bag_decl(&mut self) -> Res<()> {
        let lang = self.lang;
        self.take();
        let name = self.want_name("as the name of the members to take in")?;
        self.skip_intro();
        self.skip_seps();
        let which = lang.block_opens.iter().position(|o| self.at_lexeme(o));
        let Some(i) = which else {
            return Err(format!("Expected '{}' to open the members, got '{}'", lang.block_opens[0], self.look().lexeme));
        };
        self.take();
        self.registry.bags.insert(name, self.pos);
        // The body is stepped over, mark for mark, since it is read
        // where it is taken in and nowhere else.
        let (open, close) = (lang.block_opens[i].clone(), lang.block_closes[i].clone());
        let mut deep = 1usize;
        while deep > 0 {
            if self.exhausted() {
                return Err(format!("Expected '{}' to close the members", close));
            }
            if self.at_lexeme(&open) {
                deep += 1;
            } else if self.at_lexeme(&close) {
                deep -= 1;
            }
            self.take();
        }
        Ok(())
    }

    /// `use T, U { T::f as g; }`: the members of each bag named are read
    /// again here, standing in the class that takes them in, and any of
    /// them may be given another name in it as well.
    fn take_in_members(&mut self, held: &mut Members) -> Res<()> {
        let lang = self.lang;
        self.take();
        let apart = lang.calling.as_ref().and_then(|c| c.between.clone());
        let mut taken = Vec::new();
        loop {
            let named = self.want_name("as the members to take in")?;
            taken.push(named);
            match &apart {
                Some(sep) if self.at_symbol(sep) => self.take(),
                _ => break,
            };
        }
        for named in &taken {
            let Some(from) = self.registry.bags.get(named).copied() else {
                return Err(format!("No members named {} to take in", named));
            };
            let close = lang.block_closes[0].clone();
            let held_at = self.pos;
            self.pos = from;
            let read = self.class_members(&close, held);
            self.pos = held_at;
            read?;
        }
        // What follows may name members of the bags and give each
        // another name in this class.
        let which = lang.block_opens.iter().position(|o| self.at_lexeme(o));
        let Some(i) = which else { return Ok(()) };
        self.take();
        let close = lang.block_closes[i].clone();
        self.skip_seps();
        while !self.at_lexeme(&close) && !self.exhausted() {
            let first = self.want_name("as the member to name again")?;
            let member = match lang.scope_mark.as_ref().map_or(false, |m| self.at_symbol(m)) {
                true => {
                    self.take();
                    self.want_name("as the member to name again")?
                }
                false => first,
            };
            if !self.on_keyword(&lang.uses_alias) {
                return Err(format!("Expected '{}' after the member to name again", lang.uses_alias.first().map_or("as", String::as_str)));
            }
            self.take();
            let called = self.want_name("as the other name")?;
            let found = held.methods.iter().find(|(n, _)| *n == member).map(|(_, p)| p.clone());
            let Some(program) = found else {
                return Err(format!("No member named {} among the ones taken in", member));
            };
            held.methods.push((called, program));
            self.skip_seps();
        }
        self.want_lexeme(&close)?;
        Ok(())
    }

    /// A class and its members: properties, constants, the values it
    /// keeps for itself, and its methods. The class becomes a value
    /// bound to its name, so `new C` and `C::X` are ordinary reads.
    fn class_decl(&mut self) -> Res<()> {
        let lang = self.lang;
        let word = self.take().lexeme;
        let name = self.want_name("as the class name")?;
        // A class of method names only may stand on several at once; a
        // class stands on one and answers to any number.
        let bare = Lang::spells(&lang.interface_words, &word);
        let between = lang.calling.as_ref().and_then(|c| c.between.clone());
        let more = |a: &mut Self, into: &mut Vec<String>| -> Res<()> {
            while between.as_ref().map_or(false, |s| a.at_symbol(s)) {
                a.take();
                into.push(a.want_name("as another class named there")?);
            }
            Ok(())
        };
        let mut base = None;
        let mut answers: Vec<String> = Vec::new();
        if self.on_keyword(&lang.extends_words) {
            self.take();
            let first = self.want_name("as the class it stands on")?;
            match bare {
                true => answers.push(first),
                false => base = Some(first),
            }
            more(self, &mut answers)?;
        }
        if self.on_keyword(&lang.implements_words) {
            self.take();
            let first = self.want_name("as the class it answers to")?;
            answers.push(first);
            more(self, &mut answers)?;
        }
        // Every member's value is read into its own run of instrs, so
        // that they can be laid out in the order the plan names them.
        let (fields, shared, constants) = (Vec::new(), Vec::new(), Vec::new());
        let reaches: Vec<Reach> = Vec::new();
        let methods = Vec::new();
        let outer = self.within.replace((name.clone(), base.clone()));
        // The body may open on a line of its own, as every other block may.
        self.skip_intro();
        self.skip_seps();
        let which = lang.block_opens.iter().position(|o| self.at_lexeme(o));
        let Some(i) = which else {
            return Err(format!("Expected '{}' to open the class, got '{}'", lang.block_opens[0], self.look().lexeme));
        };
        self.take();
        let close = lang.block_closes[i].clone();
        self.skip_seps();
        // What a method keeps between calls is set where the class is
        // declared, and is set after the class is bound, since what it
        // is set to may name the class itself.
        let settings = self.mark();
        let mut held = Members { fields, shared, constants, methods, reaches };
        self.class_members(&close, &mut held)?;
        let Members { fields, shared, constants, methods, reaches } = held;
        self.want_lexeme(&close)?;
        self.within = outer;
        let kept: Vec<Instr> = self.piece().instrs.drain(settings..).collect();
        // The class it stands on first, then a value for every property,
        // every value of its own and every constant, in that order.
        let mut argc = 0;
        if let Some(base) = &base {
            let filed = self.class_key(base);
            self.read_filed(&base.clone(), &filed);
            argc += 1;
        }
        for named in &answers.clone() {
            let filed = self.class_key(named);
            self.read_filed(named, &filed);
            argc += 1;
        }
        let names = |parts: Vec<(String, Vec<Instr>)>, a: &mut Self, argc: &mut usize| {
            parts
                .into_iter()
                .map(|(member, code)| {
                    let at = a.mark();
                    for w in relocated(code, at as i64) {
                        a.put(w);
                    }
                    *argc += 1;
                    member
                })
                .collect::<Vec<String>>()
        };
        let field_names = names(fields, self, &mut argc);
        let shared_names = names(shared, self, &mut argc);
        let constant_names = names(constants, self, &mut argc);
        let plan = Plan { name: name.clone(), answers: answers.len(), field_names, field_reach: reaches, shared_names, constant_names, methods, extends: base.is_some() };
        self.act(Action::Forge(Rc::new(plan)), argc);
        let filed = self.class_key(&name);
        self.write_global(&filed);
        let at = self.mark();
        for w in relocated(kept, at as i64 - settings as i64) {
            self.put(w);
        }
        Ok(())
    }

    /// The members a class or a trait declares, read one after
    /// another until the mark that closes the body. A trait's members
    /// are read again here, at the `use` that takes them in, so that
    /// they stand in the class taking them and not in the trait.
    fn class_members(&mut self, close: &str, held: &mut Members) -> Res<()> {
        let lang = self.lang;
        while !self.at_lexeme(close) && !self.exhausted() {
            let mut own = false;
            let mut reach = Reach::Open;
            while self.look().shape == Shape::Instr {
                let w = self.look().lexeme.clone();
                if Lang::spells(&lang.shared_words, &w) {
                    own = true;
                } else if Lang::spells(&lang.hidden_words, &w) {
                    reach = Reach::Hidden;
                } else if Lang::spells(&lang.guarded_words, &w) {
                    reach = Reach::Guarded;
                } else if !Lang::spells(&lang.modifier_words, &w) {
                    break;
                }
                self.take();
            }
            if self.on_keyword(&lang.uses_words) {
                self.take_in_members(held)?;
            } else if self.on_keyword(&lang.const_words) {
                self.take();
                let member = self.want_name("as the constant name")?;
                self.expect_assign("after the constant name")?;
                held.constants.push((member, self.member_value()?));
            } else if self.on_keyword(&lang.function_words) {
                self.take();
                let gives_cell = self.skip_reference();
                let member = self.want_name("as the method name")?;
                self.giving_cells.push(gives_cell);
                let built = self.method(&member);
                self.giving_cells.pop();
                held.methods.push((member.clone(), built?));
                // A parameter of the maker that names a property makes
                // the class carry that property too.
                for named in std::mem::take(&mut self.promoted) {
                    let bare = lang.sigil.map_or(named.clone(), |s| named.trim_start_matches(s).to_string());
                    held.fields.push((bare, vec![Instr::Const(Value::Null)]));
                    held.reaches.push(Reach::Open);
                }
            } else {
                // A property, perhaps with a type word before its name.
                // One declaration may name several, written apart the
                // way a call's arguments are.
                if self.look().shape == Shape::Instr && self.look_ahead(1).shape == Shape::Instr {
                    self.take();
                }
                let apart = lang.calling.as_ref().and_then(|c| c.between.clone());
                loop {
                    let member = self.want_name("as the property name")?;
                    let bare = lang.sigil.map_or(member.clone(), |s| member.trim_start_matches(s).to_string());
                    let value = match self.on_assign() {
                        true => {
                            self.take();
                            self.member_value()?
                        }
                        false => vec![Instr::Const(Value::Null)],
                    };
                    if own {
                        held.shared.push((bare, value));
                    } else {
                        held.fields.push((bare, value));
                        held.reaches.push(reach);
                    }
                    match &apart {
                        Some(sep) if self.at_symbol(sep) => self.take(),
                        _ => break,
                    };
                }
            }
            self.skip_seps();
        }
        Ok(())
    }

    /// A member's value, read into instrs of its own that begin at nought.
    fn member_value(&mut self) -> Res<Vec<Instr>> {
        let from = self.mark();
        self.expr(0)?;
        let code: Vec<Instr> = self.piece().instrs.drain(from..).collect();
        Ok(relocated(code, -(from as i64)))
    }

    /// A method: a program whose first parameter is the object it is for,
    /// under the name the definition gives (`$this`).
    fn method(&mut self, name: &str) -> Res<Rc<Routine>> {
        self.declared_at = (self.look().row as u32).saturating_sub(self.before);
        let lang = self.lang;
        let call = lang.calling.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.want_sign(&call.open, "after method name")?;
        let this = lang.this_word.clone().ok_or_else(|| "A class needs ext.stmt.class.this".to_string())?;
        let mut formals = vec![this.clone()];
        let (params, spares, promoted) = self.parameters(name, &call)?;
        formals.extend(params);
        // The object it is for is always given, so the count is one more.
        let least = formals.len() - spares.len();
        let given = formals.clone();
        let spares: Vec<(usize, usize)> = spares.into_iter().map(|(at, from)| (at + 1, from)).collect();
        if self.look().shape == Shape::Sign && Lang::spells(&lang.return_marks, &self.look().lexeme) {
            self.take();
            if lang.annotation_marks.is_empty() {
                self.skip_nothing_mark();
                self.want_name("as a return type")?;
            } else {
                self.annotation_expression(&lang.block_intros)?;
            }
        }
        // A method may be named and not written out, in a class of
        // method names only; it answers with nothing. A body on the line
        // after the name is still a body.
        if self.on_sep() && !self.block_ahead() {
            return self.routine(name, formals, least, true, |a| {
                a.constant(Value::Null);
                a.piece().result_touched = true;
                a.write(RESULT_CELL);
                Ok(())
            });
        }
        let named = promoted.clone();
        self.promoted = promoted;
        self.routine(name, formals, least, true, |a| {
            a.spare_values(&spares, &given)?;
            // What a parameter that names a property was given is
            // written into the object before anything else runs.
            for member in &named {
                let bare = a.lang.sigil.map_or(member.clone(), |s| member.trim_start_matches(s).to_string());
                a.read(&this);
                a.read(member);
                a.act(Action::Plant(Rc::from(bare.as_str())), 2);
                a.discard();
            }
            a.body()
        })
    }

    /// The parameters of a function or a method, up to the closing bracket.
    /// A parameter with a class modifier before it names a property as
    /// well, which the maker writes what it was given into.
    fn parameters(&mut self, named: &str, call: &Brackets) -> Res<(Vec<String>, Vec<(usize, usize)>, Vec<String>)> {
        let lang = self.lang;
        let mut formals = Vec::new();
        let mut rules = Vec::new();
        let (mut named_only, mut divided, mut gather, mut pairs, mut default_seen) = (false, false, false, false, false);
        let bad = || lang.parameters_amiss.first().cloned().unwrap_or_default();
        // Which parameter, and where its own value is written: it is read
        // again inside the program, where its names mean what they should.
        let mut spares: Vec<(usize, usize)> = Vec::new();
        let mut promoted: Vec<String> = Vec::new();
        // The kind each parameter was written with, as they are read.
        let mut kinded: Vec<Option<Rc<str>>> = Vec::new();
        while !self.at_symbol(&call.close) && !self.exhausted() {
            let mut rule = if named_only { 2 } else { 0 };
            if lang.bind_names {
                if pairs { return Err(bad()); }
                let sign = self.look().lexeme.clone();
                if Lang::spells(&lang.positional_only, &sign) {
                    if divided || named_only || formals.is_empty() { return Err(bad()); }
                    divided = true;
                    rules.fill(1);
                    self.take();
                    if !self.at_symbol(&call.close) {
                        self.want_sign(call.between.as_deref().unwrap_or(""), "after positional parameters")?;
                    }
                    continue;
                }
                if Lang::spells(&lang.carries_pairs, &sign) {
                    self.take();
                    pairs = true;
                    rule = 4;
                } else if Lang::spells(&lang.carries_words, &sign) || Lang::spells(&lang.keyword_only, &sign) {
                    if gather || named_only { return Err(bad()); }
                    self.take();
                    named_only = true;
                    if call.between.as_ref().map_or(false, |sep| self.at_symbol(sep)) {
                        self.take();
                        if self.at_symbol(&call.close) || Lang::spells(&lang.carries_pairs, &self.look().lexeme) { return Err(bad()); }
                        continue;
                    }
                    gather = true;
                    rule = 3;
                }
            }
            let mut takes_nothing = false;
            let _by_cell = self.skip_reference();
            let mut names_property = false;
            while self.look().shape == Shape::Instr && Lang::spells(&lang.modifier_words, &self.look().lexeme) {
                self.take();
                names_property = true;
            }
            if lang.types_first {
                let type_word = self.want_name("as a parameter type")?;
                if !Lang::spells(&lang.let_words, &type_word) {
                    return Err(format!("'{}' is not a type word", type_word));
                }
                if self.look().shape == Shape::Instr {
                    formals.push(self.take().lexeme);
                }
            } else {
                // A type may stand before the name, and may be marked as
                // taking nothing as well: `?int $x`.
                if lang.ternary.as_ref().map_or(false, |(q, _)| self.at_symbol(q)) && self.look_ahead(1).shape == Shape::Instr {
                    self.take();
                    takes_nothing = true;
                }
                let mut kind = None;
                // The mark handing a cell over may stand between the
                // kind and the name: `foo &$a`.
                let marked = lang.reference_mark.as_ref().map_or(false, |m| self.look_ahead(1).is_lexeme(Shape::Sign, m))
                    && self.look_ahead(2).shape == Shape::Instr;
                if self.look().shape == Shape::Instr && (self.look_ahead(1).shape == Shape::Instr || marked) {
                    kind = Some(Rc::from(self.take().lexeme.as_str()));
                    if marked {
                        self.skip_reference();
                    }
                }
                self.formal_kinds.push(kind.clone());
                kinded.push(kind);
                formals.push(self.want_name("as a parameter name")?);
                if self.on_any(&lang.annotation_marks) {
                    self.take();
                    let mut ends = lang.assign_words.clone();
                    ends.push(call.close.clone());
                    ends.extend(call.between.iter().cloned());
                    self.annotation_expression(&ends)?;
                } else if self.look().shape == Shape::Sign && Lang::spells(&lang.type_marks, &self.look().lexeme) {
                    self.take();
                    self.want_name("as a type name")?;
                }
            }
            if lang.bind_names {
                let newest = formals.last().ok_or_else(bad)?;
                if formals[..formals.len() - 1].contains(newest) { return Err(bad()); }
                if self.on_assign() {
                    if rule >= 3 { return Err(bad()); }
                    if rule == 0 { default_seen = true; }
                } else if rule == 0 && default_seen { return Err(bad()); }
                rules.push(rule);
            }
            if names_property {
                promoted.push(formals.last().expect("the parameter just read").clone());
            }
            // A parameter may carry a value of its own for calls that
            // leave it out.
            if self.on_assign() {
                self.take();
                spares.push((formals.len() - 1, self.pos));
                // Read once here only to step over it.
                let spare = self.member_value()?;
                // A parameter written with a kind and given nothing to
                // fall back on takes nothing as well as that kind, which
                // the reference asks to be written out rather than left
                // to be understood.
                let nothing = matches!(spare.as_slice(), [Instr::Const(Value::Null)]);
                // A parameter written with a class's name takes a thing
                // of that class and nothing else may stand for what it
                // falls back on. Only a value written out can be told
                // apart here, and the reading stops over it as a fault
                // of the run.
                if let (Some(kind), [Instr::Const(worth)]) = (kinded.last().and_then(Clone::clone), spare.as_slice()) {
                    if !nothing && lang.names_a_class(&kind) {
                        self.registry.stopped_fatally = true;
                        let told = format!(
                            "Cannot use {} as default value for parameter {} of type {}",
                            lang.kind_word(worth),
                            formals.last().map_or("", String::as_str),
                            kind
                        );
                        return Err(told);
                    }
                }
                if nothing && !takes_nothing && kinded.last().map_or(false, Option::is_some) && lang.tells_place {
                    let whose = match &self.within {
                        Some((class, _)) => format!("{}::{}", class, named),
                        None => named.to_string(),
                    };
                    let told = format!(
                        "{}(): Implicitly marking parameter {} as nullable is deprecated, the explicit nullable type must be used instead",
                        whose,
                        formals.last().map_or("", String::as_str)
                    );
                    self.act(Action::Remark(Complaint::Deprecated, Rc::from(told.as_str())), 0);
                    self.put_away();
                }
            }
            if let Some(sep) = &call.between {
                if self.at_symbol(sep) {
                    self.take();
                } else if lang.bind_names && !self.at_symbol(&call.close) { return Err(bad()); }
            }
            if self.look().shape == Shape::Sign && lang.ends_stmt(&self.look().lexeme) {
                self.take();
            }
        }
        self.want_sign(&call.close, "after parameters")?;
        self.parameter_rules = lang.bind_names.then_some(rules);
        Ok((formals, spares, promoted))
    }

    /// An annotation is kept only long enough to find its end. No
    /// names are sought and no words for the run are made from it.
    fn annotation_expression(&mut self, ends: &[String]) -> Res<()> {
        let lang = self.lang;
        let pairs: Vec<&Brackets> = [&lang.grouping, &lang.calling, &lang.array_brackets,
            &lang.map_brackets, &lang.index_brackets].into_iter().flatten().collect();
        let mut closing: Vec<String> = Vec::new();
        let began = self.pos;
        while !self.exhausted() {
            let token = self.look();
            if closing.is_empty() && (self.on_sep() || self.on_any(ends)
                || matches!(token.shape, Shape::Open | Shape::Close)) {
                break;
            }
            if token.shape == Shape::Sign {
                if let Some(pair) = pairs.iter().find(|pair| pair.open == token.lexeme) {
                    closing.push(pair.close.clone());
                } else if pairs.iter().any(|pair| pair.close == token.lexeme) {
                    if closing.last() != Some(&token.lexeme) {
                        return Err(lang.annotation_amiss.clone().unwrap_or_else(|| "Expected an expression".into()));
                    }
                    closing.pop();
                }
            }
            self.take();
        }
        if self.pos == began || !closing.is_empty() {
            return Err(lang.annotation_amiss.clone().unwrap_or_else(|| "Expected an expression".into()));
        }
        Ok(())
    }

    /// Step over the sign saying a type takes nothing as well: `?int`.
    fn skip_nothing_mark(&mut self) {
        let marked = self.lang.ternary.as_ref().map_or(false, |(q, _)| self.at_symbol(q));
        if marked && self.look_ahead(1).shape == Shape::Instr {
            self.take();
        }
    }

    /// Step over the sign that says a name shares a cell; where it
    /// stands is already known from the reading done before compiling.
    /// The mark that says a routine gives back a cell rather than a
    /// copy, if it stands here. Whether it did is given back.
    fn skip_reference(&mut self) -> bool {
        if self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) {
            self.take();
            return true;
        }
        false
    }

    /// The instrs a program begins with when some of its parameters
    /// carry a value of their own: each is written when the call left
    /// that parameter out.
    fn spare_values(&mut self, spares: &[(usize, usize)], formals: &[String]) -> Res<()> {
        let held = self.pos;
        for (at, from) in spares {
            self.put(Instr::Missing(*at));
            let past = self.skip();
            self.pos = *from;
            self.expr(0)?;
            self.write(&formals[*at]);
            self.land(past);
        }
        self.pos = held;
        Ok(())
    }

    fn function(&mut self, name: String, gives_cell: bool) -> Res<()> {
        self.giving_cells.push(gives_cell);
        let built = self.function_body(name);
        self.giving_cells.pop();
        built
    }

    /// A routine read and left standing as a value. A name written
    /// before its brackets binds it; one written where a value stands
    /// has none, and stands for itself.
    fn function_value(&mut self, name: &str) -> Res<()> {
        let lang = self.lang;
        self.declared_at = (self.look().row as u32).saturating_sub(self.before);
        let call = lang.calling.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.want_sign(&call.open, "after function name")?;
        let (formals, spares, _) = self.parameters(&name, &call)?;
        let least = formals.len() - spares.len();
        let given = formals.clone();
        let defaults = spares.clone();
        // A routine written where a value stands may carry names from
        // around it away with it, since the names around it are gone by
        // the time it is called.
        let carried = self.carried_names(&call)?;
        let taken = carried.clone();
        if self.look().shape == Shape::Sign && Lang::spells(&lang.return_marks, &self.look().lexeme) {
            self.take();
            if lang.annotation_marks.is_empty() {
                self.skip_nothing_mark();
                self.want_name("as a return type")?;
            } else {
                self.annotation_expression(&lang.block_intros)?;
            }
        }
        let declarations = self.look().shape == Shape::Sign && lang.ends_stmt(&self.look().lexeme);
        let program = self.routine(name, formals, least, true, |a| {
            // The names carried away take the slots after the
            // parameters, and are filled from what was carried when the
            // routine is called.
            for (named, _) in &taken {
                let cell = a.cell_to_write(named);
                a.carrying.push(cell.near[0]);
            }
            if lang.bind_names {
                a.carrying.extend(spares.iter().map(|(slot, _)| *slot));
            } else {
                a.spare_values(&spares, &given)?;
            }
            if declarations {
                loop {
                    a.skip_seps();
                    if !lang.types_first && a.on_keyword(&lang.let_words) {
                        a.binding()?;
                    } else {
                        break;
                    }
                }
            }
            if lang.bind_names && Lang::spells(&lang.block_intros, &a.look().lexeme)
                && a.look_ahead(1).shape != Shape::LineEnd {
                a.take();
                a.stmt()
            } else { a.body() }
        })?;
        // What is carried is read where the routine is written, and the
        // routine takes it away with it.
        for (named, by_cell) in &carried {
            match by_cell {
                true => {
                    let cell = self.cell_to_read(named, false);
                    self.put(Instr::Bond(cell));
                }
                false => self.read(named),
            }
        }
        if lang.bind_names {
            let after = self.pos;
            for (_, from) in &defaults {
                self.pos = *from;
                self.expr(0)?;
            }
            self.pos = after;
        }
        self.constant(Value::Routine(program));
        let count = carried.len() + if lang.bind_names { defaults.len() } else { 0 };
        if count != 0 {
            self.act(Action::Close, count + 1);
        }
        Ok(())
    }

    /// An unbracketed parameter list and one expression make a value.
    /// Defaults go with that value; names in the body do not.
    fn lambda_value(&mut self) -> Res<()> {
        let lang = self.lang;
        let mark = lang.block_intros.first().cloned().ok_or("Lambda needs a body mark")?;
        let separator = lang.calling.as_ref().and_then(|b| b.between.clone()).ok_or("Lambda needs a parameter separator")?;
        let mut formals = Vec::new();
        let mut defaults = Vec::new();
        let mut rest = None;
        let mut keywords = false;
        let mut unsupported = false;
        while !self.at_symbol(&mark) {
            if self.exhausted() { return Err("Expected lambda body".to_string()); }
            if lang.dyadic.get(&self.look().lexeme).map_or(false, |op| matches!(op.action, Action::Div | Action::DivReal)) {
                self.take();
            } else {
                let star = self.look().lexeme.clone();
                let spread = lang.dyadic.get(&star).map_or(false, |op| matches!(op.action, Action::Mul | Action::Power));
                if spread {
                    self.take();
                    unsupported |= keywords || lang.dyadic.get(&star).map_or(false, |op| matches!(op.action, Action::Power));
                    keywords = true;
                    if self.at_symbol(&separator) { self.take(); unsupported = true; continue; }
                    rest = Some(formals.len());
                } else if keywords { unsupported = true; }
                let name = self.want_name("as a lambda parameter")?;
                if formals.contains(&name) { return Err("Duplicate lambda parameter".to_string()); }
                formals.push(name);
                if self.on_assign() {
                    self.take();
                    let hidden = self.gensym("default");
                    self.expr(0)?;
                    self.write(&hidden);
                    defaults.push((formals.len() - 1, hidden));
                }
            }
            if !self.at_symbol(&mark) { self.want_sign(&separator, "between lambda parameters")?; }
        }
        self.take();
        let least = rest.unwrap_or(formals.len()).saturating_sub(defaults.len());
        let given = formals.clone();
        let enclosing = if self.piece().outermost { Vec::new() } else { self.piece().idents.clone() };
        let mut unavailable = self.uncarried.clone();
        unavailable.extend(enclosing);
        let surrounding = std::mem::replace(&mut self.uncarried, unavailable);
        let mut program = self.routine(ANONYMOUS, formals, least, true, |a| {
            for (_, named) in &defaults {
                let cell = a.cell_to_write(named);
                a.carrying.push(cell.near[0]);
            }
            for (at, hidden) in &defaults {
                a.put(Instr::Missing(*at));
                let done = a.skip();
                a.read(hidden);
                a.write(&given[*at]);
                a.land(done);
            }
            let code = a.member_value()?;
            let fault = if unsupported { lang.lambda_unsupported.as_deref() } else { None };
            if let Some(told) = fault {
                a.constant(Value::text(told));
                a.act(Action::Builtin(Builtin::Raise, Rc::from("lambda")), 1);
            }
            let here = a.mark();
            for word in relocated(code, here as i64) { a.put(word); }
            a.piece().result_touched = true;
            a.write(RESULT_CELL);
            Ok(())
        })?;
        self.uncarried = surrounding;
        Rc::get_mut(&mut program).expect("a fresh lambda").rest_at = rest;
        for (_, named) in &defaults { self.read(named); }
        self.constant(Value::Routine(program));
        if !defaults.is_empty() { self.act(Action::Close, defaults.len() + 1); }
        Ok(())
    }

    /// A routine written short: its parameters, the mark, and the one
    /// expression it gives back. Every name standing around it is taken
    /// away with it as it stands, since a routine written this short has
    /// nowhere to say which of them it wants.
    fn short_value(&mut self) -> Res<()> {
        let lang = self.lang;
        let (_, mark) = lang.short_function.clone().expect("a word for a short routine");
        self.declared_at = (self.look().row as u32).saturating_sub(self.before);
        let call = lang.calling.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.want_sign(&call.open, "after the word for a short routine")?;
        let (formals, spares, _) = self.parameters(ANONYMOUS, &call)?;
        let least = formals.len() - spares.len();
        let given = formals.clone();
        if self.look().shape == Shape::Sign && Lang::spells(&lang.return_marks, &self.look().lexeme) {
            self.take();
            if lang.annotation_marks.is_empty() {
                self.skip_nothing_mark();
                self.want_name("as a return type")?;
            } else {
                self.annotation_expression(&lang.block_intros)?;
            }
        }
        self.want_sign(&mark, "before the body of a short routine")?;
        // Which names the body wants cannot be said in a routine written
        // this short, so the body is read once to find out: it is read
        // through, the names it writes are noted, what was put together
        // is thrown away, and it is read again for the routine itself.
        let began = self.pos;
        let held = self.mark();
        let wanted = {
            let spares = spares.clone();
            let given = given.clone();
            let named_here = formals.clone();
            self.routine(ANONYMOUS, formals.clone(), least, true, |a| {
                a.spare_values(&spares, &given)?;
                a.expr(0)?;
                a.piece().result_touched = true;
                a.write(RESULT_CELL);
                Ok(())
            })?;
            let ended = self.pos;
            let mut names = std::collections::BTreeSet::new();
            for tok in &self.tokens[began..ended] {
                let a_binding = tok.shape == Shape::Instr && lang.sigil.map_or(false, |m| tok.lexeme.starts_with(m));
                if a_binding && !named_here.contains(&tok.lexeme) {
                    names.insert(tok.lexeme.clone());
                }
            }
            names
        };
        self.piece().instrs.truncate(held);
        self.pos = began;
        let carried: Vec<(String, bool)> = wanted.into_iter().map(|named| (named, false)).collect();
        let taken = carried.clone();
        let program = self.routine(ANONYMOUS, formals, least, true, |a| {
            for (named, _) in &taken {
                let cell = a.cell_to_write(named);
                a.carrying.push(cell.near[0]);
            }
            a.spare_values(&spares, &given)?;
            // The one expression stands where it is written, so anything
            // said while it runs names that line and not the one the
            // call was made from.
            let row = (a.look().row as u32).saturating_sub(a.before);
            a.put(Instr::Line(row));
            a.expr(0)?;
            a.piece().result_touched = true;
            a.write(RESULT_CELL);
            Ok(())
        })?;
        // A name never written to goes with it as nothing at all: the
        // routine was never told to want it, so the taking says nothing
        // of it, and reading it inside is what says it was never
        // written, as reading it here would have been.
        for (named, _) in &carried {
            let cell = self.cell_to_read(named, false);
            self.put(Instr::Glance(cell));
        }
        self.constant(Value::Routine(program));
        if !carried.is_empty() {
            self.act(Action::Close, carried.len() + 1);
        }
        Ok(())
    }

    /// `use ($a, &$b)` after a routine written where a value stands: the
    /// names it carries away with it, and whether each is carried as it
    /// stands or as the cell it shares with the name it was read from.
    fn carried_names(&mut self, call: &Brackets) -> Res<Vec<(String, bool)>> {
        let lang = self.lang;
        if lang.carries_words.is_empty() || !self.on_keyword(&lang.carries_words) {
            return Ok(Vec::new());
        }
        self.take();
        self.want_sign(&call.open, "after the word for what a routine carries")?;
        let mut names = Vec::new();
        while !self.at_symbol(&call.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", call.close));
            }
            let by_cell = self.skip_reference();
            let named = self.want_name("as a name to carry")?;
            names.push((named, by_cell));
            if let Some(sep) = &call.between {
                if self.at_symbol(sep) {
                    self.take();
                }
            }
        }
        self.want_sign(&call.close, "after what a routine carries")?;
        Ok(names)
    }

    fn function_body(&mut self, name: String) -> Res<()> {
        self.function_value(&name)?;
        // A language may bind every routine among the outermost
        // bindings, wherever it is written, so one written inside
        // another is there for the whole run once that one has run.
        match self.lang.routines_outermost {
            true => self.write_global(&name),
            false => self.write(&name),
        }
        Ok(())
    }

    /// The target has been read as a value, as for an ordinary store.
    /// With no store, a name does nothing and an index works out only
    /// its footing and key, without asking what stands at that key.
    fn annotated_statement(&mut self, from: usize, target_at: usize) -> Res<()> {
        let lang = self.lang;
        let target_end = self.pos;
        let words = &self.piece().instrs[from..];
        let name = matches!(words, [Instr::Read(_)]);
        let index = matches!(words.last(), Some(Instr::Act(Action::At, 2)));
        let member = matches!(words.last(), Some(Instr::Act(Action::Grab(_), 1)));
        let mut brackets = 0usize;
        let mut piped = false;
        for t in &self.tokens[target_at..target_end] {
            if t.shape != Shape::Sign {
                continue;
            }
            let pairs = [&lang.calling, &lang.array_brackets, &lang.map_brackets];
            if pairs.iter().filter_map(|pair| pair.as_ref()).any(|pair| pair.open == t.lexeme) {
                brackets += 1;
            } else if pairs.iter().filter_map(|pair| pair.as_ref()).any(|pair| pair.close == t.lexeme) {
                brackets = brackets.saturating_sub(1);
            } else if brackets == 0 && Lang::spells(&lang.pipe_words, &t.lexeme) {
                piped = true;
            }
        }
        let tail: Vec<&Token> = self.tokens[target_at..target_end].iter().rev()
            .skip_while(|t| lang.grouping.as_ref().map_or(false, |pair| t.is_lexeme(Shape::Sign, &pair.close)))
            .take(2).collect();
        piped |= matches!(tail.as_slice(), [last, before] if last.shape == Shape::Instr
            && before.shape == Shape::Sign && Lang::spells(&lang.pipe_words, &before.lexeme));
        if !name && !index && !member && !piped {
            return Err(lang.annotation_amiss.clone().unwrap_or_else(|| "Expected an assignment target".into()));
        }
        self.take();
        let mut ends = lang.assign_words.clone();
        ends.extend(lang.annotation_marks.iter().cloned());
        if let Some(call) = &lang.calling {
            ends.extend(call.between.iter().cloned());
        }
        self.annotation_expression(&ends)?;
        if piped && lang.member_mark.is_none() {
            self.piece().instrs.truncate(from);
            if self.on_assign() {
                self.take();
                self.expr(0)?;
                self.put_away();
            }
            let said = lang.annotation_target_unready.as_deref().unwrap_or("This annotation target cannot be written");
            self.constant(Value::text(said));
            self.act(Action::Builtin(Builtin::Raise, Rc::from("annotation")), 1);
        } else if self.on_assign() {
            self.assignment(from, None)?;
        } else if name {
            self.piece().instrs.truncate(from);
        } else {
            self.piece().instrs.pop();
            self.put_away();
            if index {
                self.put_away();
            }
        }
        Ok(())
    }

    /// An assignment, an indexed assignment, or an expression statement.
    fn assign_or_expr(&mut self) -> Res<()> {
        // The running result is emptied before the statement is worked
        // out rather than written over after it. A slot still holding
        // what the statement before came to keeps that value alive for
        // the whole of this one, and a program asking how much room it
        // holds would be told of a value it had already finished with.
        let running = self.cell_to_write(RESULT_CELL);
        let emptied = self.put(Instr::Emptied(running));
        let from = self.mark();
        let target_at = self.pos;
        // A word left over from a head the reader does not know is not
        // the beginning of a new declaration on that same line.
        let starts_here = target_at == 0 || {
            let before = &self.tokens[target_at - 1];
            matches!(before.shape, Shape::LineEnd | Shape::Open | Shape::Close)
                || (before.shape == Shape::Sign && (self.lang.ends_stmt(&before.lexeme)
                    || Lang::spells(&self.lang.block_intros, &before.lexeme)))
        };
        self.expr_at(0, false)?;
        if self.on_any(&self.lang.tuple_marks) {
            self.scope_tail(from)?;
            if self.on_assign() { self.take(); self.scope_value()?; }
            self.piece().instrs.truncate(from);
            self.scope_fault(&self.lang.scope_unready.clone());
            return Ok(());
        }
        if starts_here && self.on_any(&self.lang.annotation_marks) {
            return self.annotated_statement(from, target_at);
        }
        // Stores through attributes or call results can be read before
        // scopes can keep them. Their fault belongs to the run.
        let dotted = matches!(&self.tokens[target_at..self.pos],
            [.., mark, name] if mark.shape == Shape::Sign && name.shape == Shape::Instr
                && Lang::spells(&self.lang.pipe_words, &mark.lexeme));
        let instructions = &self.pieces.last().expect("open piece").instrs;
        let (_, keys) = keys_apart(&instructions[from..], from, &self.keyed);
        let temporary_index = keys.first().map_or(false, |at| {
            matches!(instructions.get(at - 1), Some(Instr::Act(Action::Invoke(_), _)))
        });
        if (dotted && self.lang.member_mark.is_none() || temporary_index)
            && !self.lang.scope_unready.is_empty() && self.on_writing() {
            self.take();
            self.scope_value()?;
            self.piece().instrs.truncate(from);
            self.scope_fault(&self.lang.scope_unready.clone());
            return Ok(());
        }
        let done = if self.on_writing() {
            self.assignment(from, None)
        } else {
            self.piece().result_touched = true;
            self.write(RESULT_CELL);
            Ok(())
        };
        // A statement that never calls out of itself has no need of the
        // emptying: nothing within it can ask the run how much room it
        // holds, and what the statement before came to is let go where
        // the running result is written over anyway. The word is left
        // standing but made to do nothing, so that everything already
        // written down as an index into these words still points where
        // it did; the peephole takes it out at the end.
        if self.piece().instrs[from..].iter().all(word_alone) {
            self.piece().instrs[emptied] = Instr::Nothing;
        }
        done
    }

    /// Whether a sign that writes stands here: the assignment sign, or
    /// one of an operator and the assignment sign run together.
    fn on_writing(&self) -> bool {
        let compound = self.lang.compound.contains_key(&self.look().lexeme) && self.look().shape == Shape::Sign;
        self.on_assign() || compound
    }

    /// The value written by an assignment. Where the assignment is
    /// itself an expression, the value is kept in a cell of its own so
    /// that it can be read again once the writing is done.
    fn value_written(&mut self, keep: Option<&str>) -> Res<()> {
        // Where the value is already worked out and waiting in a cell,
        // the store reads it from there instead of reading the source
        // after the sign, since a taking-apart has no sign of its own
        // before each of its places.
        match self.waiting.clone() {
            Some(cell) => self.read(&cell),
            None => {
                if self.lang.tuple_marks.is_empty() { self.expr(0)?; } else { self.scope_value()?; }
            }
        }
        self.kept(keep);
        Ok(())
    }

    /// What a compound write takes with the value the place already
    /// holds: the source after the sign, or the one step a `++` means,
    /// which has no source of its own.
    fn addend(&mut self) -> Res<()> {
        match self.stepping {
            Some(by) => {
                self.constant(Value::Small(by));
                Ok(())
            }
            None => self.expr(0),
        }
    }

    /// The value the place holds, just read, kept where a step asked
    /// for it, so that `x++` may give back what stood there.
    fn stood_before(&mut self) {
        if let Some(cell) = self.stood.clone() {
            self.write(&cell);
            self.read(&cell);
        }
    }

    /// What a store answered with, put away: a write is not a value,
    /// so nothing of it stands where the store was written.
    fn put_away(&mut self) {
        let held = self.gensym("stored");
        self.write(&held);
    }

    fn kept(&mut self, keep: Option<&str>) {
        if let Some(cell) = keep {
            self.write(cell);
            self.read(cell);
        }
    }

    /// Turn the load of a target, already assembled from `from`, into a
    /// store of what follows the assignment sign.
    fn assignment(&mut self, from: usize, keep: Option<&str>) -> Res<()> {
        let compound = self.lang.compound.get(&self.look().lexeme).filter(|_| self.look().shape == Shape::Sign).cloned();
        let assign = self.take().lexeme;
        self.store_into(from, keep, compound, &assign)
    }

    /// The store itself, the sign that asked for it already read.
    fn store_into(&mut self, from: usize, keep: Option<&str>, compound: Option<Action>, assign: &str) -> Res<()> {
        // The target came out as a load; turn it into a store.
        let mut target: Vec<Instr> = self.piece().instrs.drain(from..).collect();
        // A target kept quiet is a store kept quiet: the marks come off
        // the load and go round the store instead.
        let hushed = matches!(target.first(), Some(Instr::Hush(true))) && matches!(target.last(), Some(Instr::Hush(false)));
        let silenced = matches!(target.first(), Some(Instr::Mute(true))) && matches!(target.last(), Some(Instr::Mute(false)));
        let mut from = from;
        if hushed || silenced {
            target = target[1..target.len() - 1].to_vec();
            self.put(match silenced {
                true => Instr::Mute(true),
                false => Instr::Hush(true),
            });
            from += 1;
        }
        // The name a method knows its own thing by is bound by the call
        // and by nothing else: a language naming one refuses a write to
        // it outright, and calls that a fault of the run.
        if let (Some(this), [Instr::Read(slot)]) = (&self.lang.this_word, target.as_slice()) {
            if slot.ident.as_ref() == this.as_str() {
                self.registry.stopped_fatally = true;
                return Err(format!("Cannot re-assign {}", this));
            }
        }
        // Where the target read its way into a place within a place,
        // the keys are taken apart, each with where it began, so that
        // the store may work each of them out once and in order.
        let (keys, key_at) = keys_apart(&target, from, &self.keyed);
        // What the chain stands on, where it is anything but the one
        // instr that reads a name: then the whole chain is rebuilt and
        // written back into whatever it stood on.
        let footing: Option<Vec<Instr>> = match key_at.first() {
            Some(at) if *at > from + 1 => Some(target[..at - from].to_vec()),
            _ => None,
        };
        let appending = matches!(target.last(), Some(Instr::Act(Action::AtEnd, 1)));
        // A slice write works out its value before it asks for bounds.
        let was_waiting = self.waiting.clone();
        if compound.is_some() && keys.iter().any(|key| matches!(key.last(), Some(Instr::Act(Action::Slice, 3)))) {
            self.act(Action::SliceUnavailable, 0);
        }
        if compound.is_none() && keys.iter().any(|key| matches!(key.last(), Some(Instr::Act(Action::Slice, 3)))) {
            let value = self.gensym("slice_value");
            self.value_written(None)?;
            self.write(&value);
            self.waiting = Some(value);
        }
        let done = match target.as_slice() {
            // `b = &a`: b is fastened to a's cell, not given a copy.
            [Instr::Read(slot)]
                if !slot.moving
                    && compound.is_none()
                    && self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) =>
            {
                let name = slot.ident.to_string();
                self.take();
                self.a_cell(&self.lang.unshared_written.clone(), false, None)?;
                let held = self.cell_to_write(&name);
                self.put(Instr::Fasten(held));
                if keep.is_some() {
                    self.read(&name);
                    self.kept(keep);
                }
                Ok(())
            }
            [Instr::Read(slot)] if !slot.moving => {
                let name = slot.ident.to_string();
                if let Some(op) = compound {
                    // x op= e is x = x op e.
                    self.read(&name);
                    self.stood_before();
                    self.addend()?;
                    self.act(op, 2);
                    self.kept(keep);
                } else {
                    self.value_written(keep)?;
                }
                self.write(&name);
                Ok(())
            }
            // `a[i][j] = v` and `a[i][] = v`: each key is worked out once
            // and in order, then the arrays along the way are rewritten
            // from the innermost outwards. A place not there yet is made
            // on the way, since that is what writing into it means.
            // A chain standing on something other than a bare name:
            // the keys are worked out once and in order, the arrays
            // along the way rewritten from the innermost outwards, and
            // the whole written back into what it stood on.
            _ if footing.is_some() => {
                let base = footing.clone().expect("what the chain stands on");
                let held: Vec<String> = (0..keys.len()).map(|_| self.gensym("key")).collect();
                for (i, key) in keys.iter().enumerate() {
                    let at = self.mark();
                    for w in relocated(key.clone(), at as i64 - key_at[i] as i64) {
                        self.put(w);
                    }
                    self.write(&held[i]);
                }
                let deep = match appending { true => keys.len(), false => keys.len() - 1 };
                let inner: Vec<String> = (0..=deep).map(|_| self.gensym("within")).collect();
                // What the chain stands on is read quietly: it is being
                // written into, and a place not there yet is made on the
                // way rather than complained about.
                // The value is worked out first, while what the chain
                // stands on is still whole.
                let value = self.gensym("value");
                if compound.is_none() {
                    self.value_written(keep)?;
                    self.write(&value);
                }
                self.put(Instr::Hush(true));
                let at = self.mark();
                for w in relocated(base.clone(), at as i64 - from as i64) {
                    self.put(w);
                }
                self.put(Instr::Hush(false));
                self.write(&inner[0]);
                for i in 0..deep {
                    self.read(&inner[i]);
                    self.read(&held[i]);
                    self.act(Action::Nested, 2);
                    self.write(&inner[i + 1]);
                }
                if let Some(op) = compound {
                    self.read(&inner[deep]);
                    self.read(&held[deep]);
                    self.act(Action::Toward, 2);
                    self.stood_before();
                    self.addend()?;
                    self.act(op, 2);
                    self.kept(keep);
                    self.write(&value);
                }
                let made = self.gensym("made");
                if appending {
                    self.read(&value);
                    self.read(&inner[deep]);
                    self.act(Action::Builtin(Builtin::Append, Rc::from("push")), 2);
                } else {
                    self.read(&held[deep]);
                    self.read(&value);
                    self.read(&inner[deep]);
                    self.act(Action::Builtin(Builtin::Replace, Rc::from("put")), 3);
                }
                self.write(&made);
                for i in (0..deep).rev() {
                    self.read(&held[i]);
                    self.read(&made);
                    self.read(&inner[i]);
                    self.act(Action::Builtin(Builtin::Replace, Rc::from("put")), 3);
                    self.write(&made);
                }
                // What it stood on is written back into, read again as
                // the store it is.
                let footing_at = self.mark();
                for w in relocated(base, footing_at as i64 - from as i64) {
                    self.put(w);
                }
                let was = self.waiting.replace(made);
                let stored = self.store_into(footing_at, None, None, "=");
                self.waiting = was;
                stored
            }
            [Instr::Read(slot), ..]
                if !slot.moving && (keys.len() > 1 || (keys.len() == 1 && (appending || compound.is_some()))) =>
            {
                let name = slot.ident.to_string();
                let held: Vec<String> = (0..keys.len()).map(|_| self.gensym("key")).collect();
                for (i, key) in keys.iter().enumerate() {
                    let at = self.mark();
                    for w in relocated(key.clone(), at as i64 - key_at[i] as i64) {
                        self.put(w);
                    }
                    self.write(&held[i]);
                }
                // Down through the arrays, keeping each one, as far as
                // the one the write itself lands in. The value comes
                // first, while the array is still where it stands, since
                // reading it out to rewrite it would leave the name
                // holding nothing while the value is worked out.
                let deep = match appending { true => keys.len(), false => keys.len() - 1 };
                let inner: Vec<String> = (0..=deep).map(|_| self.gensym("within")).collect();
                let value = self.gensym("value");
                if compound.is_none() {
                    self.value_written(keep)?;
                    self.write(&value);
                }
                self.read_to_rewrite(&name);
                self.write(&inner[0]);
                for i in 0..deep {
                    self.read(&inner[i]);
                    self.read(&held[i]);
                    self.act(Action::Nested, 2);
                    self.write(&inner[i + 1]);
                }
                // x op= e writes what x holds now, taken with e, which
                // wants the place read first and so comes after.
                if let Some(op) = compound {
                    self.read(&inner[deep]);
                    self.read(&held[deep]);
                    self.act(Action::Toward, 2);
                    self.stood_before();
                    self.addend()?;
                    self.act(op, 2);
                    self.kept(keep);
                    self.write(&value);
                }
                // Back out again, each array rewritten in the one above.
                let made = self.gensym("made");
                if appending {
                    self.read(&value);
                    self.read(&inner[deep]);
                    self.act(Action::Builtin(Builtin::Append, Rc::from("push")), 2);
                } else {
                    self.read(&held[deep]);
                    self.read(&value);
                    self.read(&inner[deep]);
                    self.act(Action::Builtin(Builtin::Replace, Rc::from("put")), 3);
                }
                self.write(&made);
                for i in (0..deep).rev() {
                    self.read(&held[i]);
                    self.read(&made);
                    self.read(&inner[i]);
                    self.act(Action::Builtin(Builtin::Replace, Rc::from("put")), 3);
                    self.write(&made);
                }
                self.read(&made);
                self.write(&name);
                Ok(())
            }
            // `p op= e` where the place is a member of something: what
            // holds it is worked out once and kept, the member read
            // from there, taken with the value, and what comes of it
            // written back into the same place.
            [rest @ .., Instr::Act(Action::Grab(member), 1)] if compound.is_some() => {
                let (member, rest) = (member.clone(), rest.to_vec());
                let op = compound.expect("the operation");
                let holder = self.gensym("holder");
                for w in relocated(rest, 0) {
                    self.put(w);
                }
                self.write(&holder);
                self.read(&holder);
                self.act(Action::Grab(member.clone()), 1);
                self.stood_before();
                self.addend()?;
                self.act(op, 2);
                self.kept(keep);
                let value = self.gensym("value");
                self.write(&value);
                self.read(&holder);
                self.read(&value);
                self.act(Action::Plant(member), 2);
                self.put_away();
                Ok(())
            }
            // The same for a class's own value.
            [rest @ .., Instr::Act(Action::Reach(member), 1)] if compound.is_some() => {
                let (member, rest) = (member.clone(), rest.to_vec());
                let op = compound.expect("the operation");
                let holder = self.gensym("holder");
                for w in relocated(rest, 0) {
                    self.put(w);
                }
                self.write(&holder);
                self.read(&holder);
                self.act(Action::Reach(member.clone()), 1);
                self.stood_before();
                self.addend()?;
                self.act(op, 2);
                self.kept(keep);
                let value = self.gensym("value");
                self.write(&value);
                self.read(&holder);
                self.read(&value);
                self.act(Action::Sow(member), 2);
                self.put_away();
                Ok(())
            }
            // `$$x op= e`: the name is worked out once and kept, the
            // binding it spells read under that name, taken with the
            // value, and what comes of it written back under it.
            [rest @ .., Instr::Act(Action::Named, 1)] if compound.is_some() => {
                let rest = rest.to_vec();
                let op = compound.expect("the operation");
                let spelling = self.gensym("spelling");
                for w in relocated(rest, 0) {
                    self.put(w);
                }
                self.write(&spelling);
                self.read(&spelling);
                self.act(Action::Named, 1);
                self.stood_before();
                self.addend()?;
                self.act(op, 2);
                self.kept(keep);
                let value = self.gensym("value");
                self.write(&value);
                self.read(&spelling);
                self.read(&value);
                self.act(Action::WriteNamed, 2);
                self.put_away();
                Ok(())
            }
            _ if compound.is_some() => Err(format!("'{}' needs a plain variable on its left", assign)),
            // `$GLOBALS['n'][] = v`: the binding the name spells is read
            // quietly, since a write makes what is not there yet, the
            // value put after its last place, and the whole written back
            // under the same name.
            [rest @ .., Instr::Act(Action::Named, 1), Instr::Act(Action::AtEnd, 1)] if compound.is_none() => {
                let rest = rest.to_vec();
                let named = self.gensym("named");
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.write(&named);
                self.read(&named);
                self.value_written(keep)?;
                self.put(Instr::Hush(true));
                self.read(&named);
                self.act(Action::Named, 1);
                self.put(Instr::Hush(false));
                self.act(Action::Builtin(Builtin::Append, Rc::from("push")), 2);
                self.act(Action::WriteNamed, 2);
                self.put_away();
                Ok(())
            }
            // A read of the binding a value names turns into a write
            // of it: the text stays where it is and the value follows.
            // `$GLOBALS['n'] = &e`: the binding the name spells is
            // fastened to the cell, as a name written out would be.
            [rest @ .., Instr::Act(Action::Named, 1)]
                if compound.is_none() && self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) =>
            {
                for w in relocated(rest.to_vec(), 0) {
                    self.put(w);
                }
                self.take();
                self.a_cell(&self.lang.unshared_written.clone(), false, None)?;
                self.act(Action::FastenNamed, 2);
                self.put_away();
                Ok(())
            }
            [rest @ .., Instr::Act(Action::Named, 1)] if compound.is_none() => {
                for w in relocated(rest.to_vec(), 0) {
                    self.put(w);
                }
                self.value_written(keep)?;
                self.act(Action::WriteNamed, 2);
                self.put_away();
                Ok(())
            }
            // The read of a class's own value named by a value turns
            // into a write of it, the class and the name staying put.
            [rest @ .., Instr::Act(Action::ReachNamed, 2)] => {
                let rest = rest.to_vec();
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.value_written(keep)?;
                self.act(Action::SowNamed, 3);
                self.put_away();
                Ok(())
            }
            // The read of a member named by a value turns into a write
            // of it: what it belongs to and the name both stay where
            // they are, and the value follows them.
            [rest @ .., Instr::Act(Action::GrabNamed, 2)] => {
                let rest = rest.to_vec();
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.value_written(keep)?;
                self.act(Action::PlantNamed, 3);
                self.put_away();
                Ok(())
            }
            // The read of a member turns into a write of it.
            [rest @ .., Instr::Act(Action::Grab(member), 1)] => {
                let (member, rest) = (member.clone(), rest.to_vec());
                for w in relocated(rest, 0) {
                    self.put(w);
                }
                self.value_written(keep)?;
                self.act(Action::Plant(member), 2);
                self.put_away();
                Ok(())
            }
            [rest @ .., Instr::Act(Action::Reach(member), 1)] => {
                let (member, rest) = (member.clone(), rest.to_vec());
                for w in relocated(rest, 0) {
                    self.put(w);
                }
                self.value_written(keep)?;
                self.act(Action::Sow(member), 2);
                self.put_away();
                Ok(())
            }
            // `$o->a[i] = v` and `$o->a[] = v`: the property is read, the
            // array within it rewritten, and the property written back.
            [Instr::Read(slot), Instr::Act(Action::Grab(member), 1), index @ ..]
                if !slot.moving
                    && matches!(index.last(), Some(Instr::Act(Action::At, 2)) | Some(Instr::Act(Action::AtEnd, 1))) =>
            {
                let name = slot.ident.to_string();
                let (member, index) = (member.clone(), index.to_vec());
                let appending = matches!(index.last(), Some(Instr::Act(Action::AtEnd, 1)));
                self.read(&name);
                if !appending {
                    for w in relocated(index[..index.len() - 1].to_vec(), self.mark() as i64 - 2) {
                        self.put(w);
                    }
                }
                self.value_written(keep)?;
                self.read(&name);
                self.act(Action::Grab(member.clone()), 1);
                let native = if appending { Builtin::Append } else { Builtin::Replace };
                let argc = if appending { 2 } else { 3 };
                self.act(Action::Builtin(native, Rc::from("put")), argc);
                self.act(Action::Plant(member), 2);
                self.put_away();
                Ok(())
            }
            [Instr::Read(slot), Instr::Act(Action::AtEnd, 1)] if !slot.moving => {
                // a[] = v appends.
                let name = slot.ident.to_string();
                self.value_written(keep)?;
                self.read_to_rewrite(&name);
                self.act(Action::Builtin(Builtin::Append, Rc::from("push")), 2);
                self.rewritten(&name);
                Ok(())
            }
            // `a[i] = &b`: the place holds the cell itself, so a write
            // through either name is a write the other sees.
            [Instr::Read(slot), index @ .., Instr::Act(Action::At, 2)]
                if !slot.moving
                    && compound.is_none()
                    && self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) =>
            {
                let name = slot.ident.to_string();
                for w in relocated(index.to_vec(), self.mark() as i64 - from as i64 - 1) {
                    self.put(w);
                }
                self.take();
                self.a_cell(&self.lang.unshared_written.clone(), false, None)?;
                self.read_to_rewrite(&name);
                self.act(Action::Builtin(Builtin::Replace, Rc::from("put")), 3);
                self.rewritten(&name);
                if keep.is_some() {
                    self.constant(Value::Null);
                    self.kept(keep);
                }
                Ok(())
            }
            [Instr::Read(slot), index @ .., Instr::Act(Action::At, 2)] if !slot.moving => {
                let name = slot.ident.to_string();
                for w in relocated(index.to_vec(), self.mark() as i64 - from as i64 - 1) {
                    self.put(w);
                }
                // Where a language writes into text, only the thing
                // written into can say whether this place holds a
                // letter, so it is read before the value is put away:
                // what such a write is worth is the letter that went
                // in and not the whole of what was handed over.
                if self.lang.text_places {
                    self.value_written(None)?;
                    self.read_to_rewrite(&name);
                    self.act(Action::Fitted, 1);
                    if keep.is_some() {
                        let aside = self.gensym("into");
                        self.write(&aside);
                        self.kept(keep);
                        self.read_to_rewrite(&aside);
                    }
                } else {
                    self.value_written(keep)?;
                    self.read_to_rewrite(&name);
                }
                self.act(Action::Builtin(Builtin::Replace, Rc::from("put")), 3);
                self.rewritten(&name);
                Ok(())
            }
            _ => Err(format!("Invalid assignment target before '{}'", assign)),
        };
        self.waiting = was_waiting;
        if hushed || silenced {
            self.put(match silenced {
                true => Instr::Mute(false),
                false => Instr::Hush(false),
            });
        }
        done
    }

    // ---------- expressions ----------

    fn expr(&mut self, floor: u32) -> Res<()> {
        self.expr_at(floor, true)
    }

    /// Whether all that has been laid down from here is one word an
    /// operation can read for itself: a name or a number wants no
    /// holding place, so nothing stands between it and the operation.
    fn lone_operand(&mut self, from: usize) -> bool {
        let instrs = &self.piece().instrs;
        instrs.len() == from + 1 && operand_of(&instrs[from]).is_some()
    }

    /// An expression. Where a language counts an assignment as one, a
    /// target followed by a sign that writes is read as an assignment
    /// whose value is what was written — but not where the assignment
    /// is the whole statement, which is read as a statement.
    fn expr_at(&mut self, floor: u32, may_write: bool) -> Res<()> {
        let lang = self.lang;
        let from = self.mark();
        if floor == 0 && self.look().shape == Shape::Instr && Lang::spells(&lang.expression_assign, &self.look_ahead(1).lexeme) {
            let named = self.take().lexeme;
            self.take();
            self.cell_to_write(&named);
            self.expr(0)?;
            self.write(&named);
            self.read(&named);
            return Ok(());
        }
        self.prefix()?;
        if floor == 0 && Lang::spells(&lang.expression_assign, &self.look().lexeme) {
            let name = match &self.piece().instrs[from..] {
                [Instr::Read(cell)] => cell.ident.to_string(),
                _ => return Err("Named expression needs a variable".to_string()),
            };
            self.piece().instrs.truncate(from);
            self.take();
            self.expr(0)?;
            self.write(&name);
            self.read(&name);
            return Ok(());
        }
        if floor == 0 && may_write && lang.assign_gives_value && self.on_writing() {
            let keep = self.gensym("written");
            self.assignment(from, Some(&keep))?;
            self.read(&keep);
            return Ok(());
        }
        loop {
            let t = self.look();
            if !matches!(t.shape, Shape::Sign | Shape::Instr) {
                break;
            }
            let text = t.lexeme.clone();
            if floor == 0 && lang.if_else_words.first() == Some(&text) {
                self.take();
                let yes: Vec<Instr> = self.piece().instrs.drain(from..).collect();
                self.expr(1)?;
                let no = self.skip();
                let here = self.mark();
                for word in relocated(yes, here as i64 - from as i64) {
                    self.put(word);
                }
                let done = self.leap();
                self.land(no);
                let other = lang.if_else_words.get(1).ok_or("Conditional expression needs two words")?;
                if !self.at_lexeme(other) {
                    return Err(format!("Expected '{}' in conditional expression", other));
                }
                self.take();
                self.expr(0)?;
                self.land(done);
                continue;
            }
            if lang.chained_comparisons {
                if let Some((op, tier, width)) = self.comparison() {
                    if tier < floor { break; }
                    self.comparisons(op, tier, width)?;
                    continue;
                }
            }
            if Lang::spells(&lang.pipe_words, &text) {
                if lang.precedence.get(&text).copied().unwrap_or(0) < floor {
                    break;
                }
                self.take();
                self.pipe_target(from)?;
                continue;
            }
            if lang.otherwise_mark.as_ref().map_or(false, |m| self.at_symbol(m)) {
                // `a ?? b`: b is worked out only when a is nothing. What
                // stands on the left is read quietly, since a name or a
                // place that is not there is the very case the whole is
                // written for, and is nothing to complain of.
                self.take();
                let left: Vec<Instr> = self.piece().instrs.drain(from..).collect();
                self.put(Instr::Hush(true));
                let at = self.mark();
                for w in relocated(left, at as i64 - from as i64) {
                    self.put(w);
                }
                self.put(Instr::Hush(false));
                self.write(TEMP_CELL);
                self.read(TEMP_CELL);
                self.act(Action::Nothing, 1);
                let done = self.skip();
                self.expr(0)?;
                self.write(TEMP_CELL);
                self.land(done);
                self.read(TEMP_CELL);
                continue;
            }
            if self.on_keyword(&lang.instanceof_words) {
                self.take();
                let named = self.want_name("as the class to test against")?;
                // The class to test against is usually written out. A
                // binding written there stands for a class as the run
                // reaches it: the class it spells, or the class of the
                // thing it holds.
                if lang.sigil.map_or(false, |mark| named.starts_with(mark)) {
                    self.read(&named);
                    self.act(Action::KindredTo, 2);
                    continue;
                }
                self.act(Action::Kindred(Rc::from(named.as_str())), 1);
                continue;
            }
            let comparison = self.comparison();
            let Some(mut infix) = lang.dyadic.get(&text).cloned().or_else(|| {
                comparison.as_ref().map(|(op, level, _)| crate::lang::Operator { action: op.clone(), level: *level, right_assoc: false })
            }) else {
                if floor == 0 && lang.ternary.as_ref().map_or(false, |(q, _)| self.at_symbol(q)) {
                    self.ternary()?;
                    continue;
                }
                break;
            };
            if infix.level < floor {
                break;
            }
            let width = match comparison {
                Some((op, _, width)) => { infix.action = op; width }
                None => 1,
            };
            for _ in 0..width { self.take(); }
            let right_floor = if infix.right_assoc { infix.level } else { infix.level + 1 };
            match infix.action {
                Action::And | Action::Or => {
                    // The right side runs only when the left leaves it open.
                    self.write(TEMP_CELL);
                    self.read(TEMP_CELL);
                    if matches!(infix.action, Action::Or) {
                        self.act(Action::Not, 1);
                    }
                    let skip = self.skip();
                    self.expr(right_floor)?;
                    self.write(TEMP_CELL);
                    self.land(skip);
                    self.read(TEMP_CELL);
                    self.act(Action::AsBool, 1);
                }
                op => {
                    // A lone name is read where the operator falls and
                    // not where it stands, so a write on the other side
                    // is seen by it. Only a name is fetched that late: a
                    // property, a class's own value, a place in an array
                    // or what a call gave back is put in a holding place
                    // as it stands, and a write cannot reach it there.
                    let named = match self.piece().instrs[from..] {
                        [Instr::Read(ref cell)] if !cell.moving => Some(cell.clone()),
                        _ => None,
                    };
                    if named.is_some() {
                        self.piece().instrs.truncate(from);
                    }
                    self.expr(right_floor)?;
                    match named {
                        // Where the other side is itself a name or a
                        // number the reading was late already: the
                        // peephole takes both straight from the word.
                        Some(cell) if self.lone_operand(from) => {
                            self.piece().instrs.insert(from, Instr::Read(cell));
                        }
                        // Otherwise what the other side came to is
                        // stowed and the name read after it, the two
                        // reads folding back into the operation. The
                        // name is looked for again there, since the
                        // other side may be what first bound it.
                        Some(cell) => {
                            self.write(TEMP_CELL);
                            self.read(&cell.ident);
                            self.read(TEMP_CELL);
                        }
                        None => {}
                    }
                    self.act(op, 2);
                }
            }
        }
        Ok(())
    }

    fn comparison(&self) -> Option<(Action, u32, usize)> {
        let word = &self.look().lexeme;
        if Lang::spells(&self.lang.membership_not, word) {
            let next = &self.look_ahead(1).lexeme;
            if Lang::spells(&self.lang.membership_words, next) {
                return self.lang.dyadic.get(next).map(|op| (Action::Lacks, op.level, 2));
            }
        }
        let op = self.lang.dyadic.get(word)?;
        let mut action = op.action.clone();
        let mut width = 1;
        if matches!(action, Action::Same) && Lang::spells(&self.lang.identity_not, &self.look_ahead(1).lexeme) {
            action = Action::Unsame;
            width = 2;
        }
        matches!(action, Action::Eq | Action::Ne | Action::Lt | Action::Le | Action::Gt | Action::Ge | Action::Same | Action::Unsame | Action::Contains | Action::Lacks)
            .then_some((action, op.level, width))
    }

    /// Each link keeps its far side for the link after it. A false link
    /// goes straight to the end, leaving the rest untouched.
    fn comparisons(&mut self, mut op: Action, tier: u32, mut width: usize) -> Res<()> {
        let near = self.gensym("near");
        let far = self.gensym("far");
        let answer = self.gensym("comparison");
        self.write(&near);
        let mut exits = Vec::new();
        loop {
            for _ in 0..width { self.take(); }
            self.expr(tier + 1)?;
            self.write(&far);
            self.read(&near);
            self.read(&far);
            self.act(op, 2);
            self.write(&answer);
            let Some((next, level, words)) = self.comparison().filter(|(_, level, _)| *level == tier) else { break; };
            let _ = level;
            self.read(&answer);
            exits.push(self.skip());
            self.read(&far);
            self.write(&near);
            op = next;
            width = words;
        }
        for exit in exits { self.land(exit); }
        self.read(&answer);
        Ok(())
    }

    /// After a pipe: a call with the piped value first, or a bare name,
    /// a call with no other argument.
    fn pipe_target(&mut self, left: usize) -> Res<()> {
        let name = self.want_name("after the pipe")?;
        let mut ahead = 0;
        while self.lang.grouping.as_ref().map_or(false, |pair| self.look_ahead(ahead).is_lexeme(Shape::Sign, &pair.close)) {
            ahead += 1;
        }
        if self.look_ahead(ahead).shape == Shape::Sign
            && Lang::spells(&self.lang.annotation_marks, &self.look_ahead(ahead).lexeme) {
            // The statement reader will speak of the unsupported place;
            // its member name must not be mistaken for a builtin call.
            self.read(&name);
            return Ok(());
        }
        let native = self.lang.builtins.get(&name).copied();
        if matches!(native, Some(Builtin::Append) | Some(Builtin::Replace)) {
            // arr.push(x): the piped value must be the array's name.
            let target = match &self.piece().instrs[left..] {
                [Instr::Read(slot)] if !slot.moving => slot.ident.to_string(),
                _ => return Err(format!("First argument to {}() must be an array variable name", name)),
            };
            self.piece().instrs.truncate(left);
            let Some(call) = self.lang.calling.clone() else {
                return Err("This language has no call syntax".to_string());
            };
            self.want_sign(&call.open, "after the method name")?;
            let argc = self.arguments(&call)?;
            return self.mutation(&name, &target, argc + 1);
        }
        let mut argc = 1;
        if let Some(call) = self.lang.calling.clone() {
            if self.at_symbol(&call.open) {
                self.take();
                argc += self.arguments(&call)?;
            }
        }
        self.call(&name, argc)
    }

    /// `test ? a : b`, the test just assembled: one arm's value is left.
    fn ternary(&mut self) -> Res<()> {
        let (_, mark) = self.lang.ternary.clone().expect("the ternary signs");
        self.take();
        let skip = self.skip();
        self.expr(0)?;
        let over = self.leap();
        self.land(skip);
        self.want_sign(&mark, "between the arms of the conditional")?;
        self.expr(0)?;
        self.land(over);
        Ok(())
    }

    /// `++p` and `p--` over any place a write reaches: the place is
    /// read, stepped by one and written back. What stands afterwards is
    /// the value after the step, or the one that stood there before it
    /// where the step is written after the place.
    fn bumped(&mut self, from: usize, op: Action, gives_new: bool) -> Res<()> {
        let stood = (!gives_new).then(|| self.gensym("stood"));
        let keep = gives_new.then(|| self.gensym("stepped"));
        let was_step = self.stepping.replace(1);
        let was_stood = std::mem::replace(&mut self.stood, stood.clone());
        let done = self.store_into(from, keep.as_deref(), Some(op), "++");
        self.stepping = was_step;
        self.stood = was_stood;
        done?;
        if let Some(cell) = keep.or(stood) {
            self.read(&cell);
        }
        Ok(())
    }

    /// A piece of an expression, with a step written before or after it
    /// where the language spells one.
    fn prefix(&mut self) -> Res<()> {
        let from = self.mark();
        if let Some(op) = self.bump_of(&self.look().clone()) {
            self.take();
            self.prefix()?;
            return self.bumped(from, op, true);
        }
        self.prefix_piece()?;
        if let Some(op) = self.bump_of(&self.look().clone()) {
            self.take();
            return self.bumped(from, op, false);
        }
        Ok(())
    }

    fn string_piece(&mut self) -> Res<()> {
        let token = self.take();
        match token.shape {
            Shape::Quote => self.constant(Value::text(&token.lexeme)),
            Shape::StringFault => {
                self.constant(Value::text(&token.lexeme));
                self.act(Action::StringFault, 1);
            }
            Shape::StringBegin => {
                self.constant(Value::text(""));
                while self.look().shape != Shape::StringEnd {
                    if self.look().shape == Shape::StringField {
                        let field = self.take();
                        self.expr(0)?;
                        self.string_piece()?;
                        self.constant(Value::text(&field.lexeme));
                        self.act(Action::StringRender, 3);
                    } else { self.string_piece()?; }
                    self.act(Action::Join, 2);
                }
                self.take();
            }
            _ => return Err(self.lang.string_amiss.clone().unwrap_or_else(|| "Invalid string literal".into())),
        }
        Ok(())
    }

    fn prefix_piece(&mut self) -> Res<()> {
        let lang = self.lang;
        let from = self.mark();
        let tok = self.look().clone();
        if self.yield_operand && lang.dyadic.get(&tok.lexeme).map_or(false, |op| matches!(op.action, Action::Mul)) {
            self.take();
            return self.prefix();
        }
        if self.on_keyword(&lang.await_words) {
            self.take();
            return self.prefix();
        }
        if self.on_keyword(&lang.yield_words) {
            self.take();
            self.piece().generator = true;
            let delegated = self.on_keyword(&lang.yield_from_words);
            if delegated { self.take(); }
            let begin = self.mark();
            let outer_operand = std::mem::replace(&mut self.yield_operand, true);
            let ended = |r: &Self| r.on_sep() || r.exhausted() || r.look().shape == Shape::Close
                || r.lang.grouping.as_ref().map_or(false, |g| r.at_symbol(&g.close))
                || r.lang.array_brackets.as_ref().map_or(false, |g| r.at_symbol(&g.close));
            if delegated || !ended(self) {
                loop {
                    self.expr(0)?;
                    if delegated || !lang.calling.as_ref().and_then(|c| c.between.as_ref()).map_or(false, |s| self.at_symbol(s)) { break; }
                    self.take();
                    if ended(self) { break; }
                }
            }
            self.yield_operand = outer_operand;
            self.piece().instrs.truncate(begin);
            self.constant(Value::text(lang.yield_unrun.first().map_or("", String::as_str)));
            self.act(Action::Builtin(Builtin::Raise, Rc::from("")), 1);
            return Ok(());
        }
        if Lang::spells(&lang.ellipsis_words, &tok.lexeme) {
            self.take();
            self.constant(Value::Ellipsis);
            return self.indexing(from);
        }
        if Lang::spells(&lang.lambda_words, &tok.lexeme) {
            self.take();
            self.lambda_value()?;
            return Ok(());
        }
        // `list($a, $b) = v`: the places named on the left each take
        // the matching place of the value on the right.
        if Lang::spells(&lang.unpack_words, &tok.lexeme) && matches!(tok.shape, Shape::Instr | Shape::Sign) {
            self.take();
            // The value is worked out first and kept, since every place
            // reads from the one value.
            let holding = self.gensym("taken");
            let places = self.pos;
            self.skip_past_call()?;
            let sign = self.take();
            if !self.lang.assign_words.iter().any(|w| *w == sign.lexeme) {
                return Err(format!("A taking-apart must be written on the left of a write, not '{}'", sign.lexeme));
            }
            self.expr(0)?;
            self.write(&holding);
            let after = self.pos;
            self.pos = places;
            self.unpack(&holding)?;
            self.pos = after;
            return Ok(());
        }
        // `(int) x`: a kind's name written within the grouping marks
        // before a value makes the value that kind. Only a word the
        // language names a kind by counts, so a plain grouping of a
        // name is still a grouping.
        if lang.casts_kinds {
            if let Some(group) = lang.grouping.clone() {
                let kind = self
                    .at_symbol(&group.open)
                    .then(|| lang.kind_of_word(&self.look_ahead(1).lexeme))
                    .flatten()
                    .filter(|_| self.look_ahead(2).is_lexeme(Shape::Sign, &group.close));
                if let Some(kind) = kind {
                    self.take();
                    self.take();
                    self.take();
                    let tier = lang.monadic.values().map(|m| m.level).max().unwrap_or(0);
                    self.expr(tier)?;
                    self.act(Action::Cast(kind), 1);
                    return Ok(());
                }
            }
        }
        if matches!(tok.shape, Shape::Sign | Shape::Instr) {
            if let Some(infix) = lang.monadic.get(&tok.lexeme).cloned() {
                self.take();
                self.expr(infix.level)?;
                self.act(infix.action, 1);
                return Ok(());
            }
            if Lang::spells(&lang.naming_words, &tok.lexeme) {
                // `$$a` and `${e}`: the name is what the value spells.
                // Only the outermost bindings have names the run can
                // still see, so a name worked out inside a unit of its
                // own is refused rather than quietly meaning another.
                if !self.piece().outermost {
                    return Err(format!("'{}' works a name out while the program runs, which only the outermost bindings have", tok.lexeme));
                }
                self.take();
                self.naming()?;
                self.act(Action::Named, 1);
                return self.indexing(from);
            }
            if Lang::spells(&lang.hush_words, &tok.lexeme) {
                // Whatever the piece under the mark has to say about
                // itself is kept quiet; its value stands as it would.
                let tier = lang.precedence.get(&tok.lexeme).copied().unwrap_or(0);
                self.take();
                self.put(Instr::Mute(true));
                self.expr(tier)?;
                self.put(Instr::Mute(false));
                return Ok(());
            }
            if tok.shape == Shape::Sign && Lang::spells(&lang.plus_words, &tok.lexeme) {
                // A plus sign leaves its operand alone, bound as tightly as a negation.
                self.take();
                let tier = lang.monadic.values().map(|m| m.level).max().unwrap_or(0);
                return self.expr(tier);
            }
        }
        match tok.shape {
            Shape::Numeral => {
                self.take();
                let v = parse_number(&tok.lexeme, lang)?;
                self.constant(v);
            }
            Shape::Quote | Shape::StringBegin | Shape::StringFault => {
                self.string_piece()?;
                while lang.adjacent_strings && matches!(self.look().shape, Shape::Quote | Shape::StringBegin | Shape::StringFault) {
                    self.string_piece()?;
                    self.act(Action::Join, 2);
                }
            }
            Shape::Instr if Lang::spells(&lang.new_words, &tok.lexeme) => {
                self.take();
                let named = self.want_name("as the class to make")?;
                self.read_class(&named)?;
                self.class_reference()?;
                // A maker takes its arguments as any other routine does,
                // so a parameter of it that takes a cell is handed one.
                let maker = lang.constructor.clone().unwrap_or_default();
                let argc = match lang.calling.clone() {
                    Some(call) if self.at_symbol(&call.open) => {
                        self.take();
                        self.arguments_of(&maker, &call)?
                    }
                    _ => 0,
                };
                self.act(Action::Make, argc + 1);
            }
            Shape::Instr if Lang::spells(&lang.self_words, &tok.lexeme) || Lang::spells(&lang.parent_words, &tok.lexeme) => {
                self.take();
                self.read_class(&tok.lexeme)?;
            }
            Shape::Instr => {
                self.take();
                if Lang::spells(&lang.true_words, &tok.lexeme) {
                    self.constant(Value::Flag(true));
                } else if Lang::spells(&lang.false_words, &tok.lexeme) {
                    self.constant(Value::Flag(false));
                } else if Lang::spells(&lang.null_words, &tok.lexeme) {
                    self.constant(Value::Null);
                } else if lang.short_function.as_ref().map_or(false, |(word, _)| word == &tok.lexeme)
                    && lang.calling.as_ref().map_or(false, |c| self.at_symbol(&c.open))
                {
                    // A routine written short is one expression, and
                    // takes with it every name standing around it: what
                    // it wants of them cannot be said, so it takes them
                    // all as they stand.
                    self.short_value()?;
                } else if Lang::spells(&lang.function_words, &tok.lexeme)
                    && lang
                        .calling
                        .as_ref()
                        .map_or(false, |c| self.at_symbol(&c.open) || (lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) && self.look_ahead(1).is_lexeme(Shape::Sign, &c.open)))
                {
                    // A routine written where a value stands has no
                    // name to be bound to and stands for itself. It
                    // reaches none of the names around it, only the
                    // outermost ones, as a routine written out does.
                    let gives_cell = self.skip_reference();
                    self.giving_cells.push(gives_cell);
                    let built = self.function_value(ANONYMOUS);
                    self.giving_cells.pop();
                    built?;
                } else {
                    match lang.calling.clone() {
                        Some(call) if self.at_symbol(&call.open) => {
                            self.take();
                            let native = lang.builtins.get(&tok.lexeme).copied();
                            if matches!(native, Some(Builtin::Append) | Some(Builtin::Replace)) {
                                // push(arr, v), put(arr, i, v): the array is named.
                                let target = match self.look().shape {
                                    Shape::Instr => self.take().lexeme,
                                    _ => return Err(format!("First argument to {}() must be an array variable name", tok.lexeme)),
                                };
                                if let Some(sep) = &call.between {
                                    if self.at_symbol(sep) {
                                        self.take();
                                    }
                                }
                                let argc = self.arguments(&call)?;
                                self.mutation(&tok.lexeme, &target, argc + 1)?;
                            } else if native == Some(Builtin::Lead) {
                                // The array put in front of is named, as
                                // the one pushed onto is.
                                let target = match self.look().shape {
                                    Shape::Instr => self.take().lexeme,
                                    _ => return Err(format!("First argument to {}() must be an array variable name", tok.lexeme)),
                                };
                                if let Some(sep) = &call.between {
                                    if self.at_symbol(sep) {
                                        self.take();
                                    }
                                }
                                let argc = self.arguments(&call)?;
                                self.leading(&tok.lexeme, &target, argc)?;
                            } else if native == Some(Builtin::Erase) {
                                self.forget(&call)?;
                            } else if native == Some(Builtin::Held) {
                                self.held(&call)?;
                            } else if native == Some(Builtin::Hollow) {
                                self.hollow(&call)?;
                            } else if native == Some(Builtin::Pack) {
                                // array(...) gathers its arguments like a literal.
                                let count = self.elements(&call)?;
                                self.act(Action::MakeArray, count);
                            } else if native == Some(Builtin::Define) {
                                // define("NAME", v) binds the global NAME here; its value is true.
                                let from = self.mark();
                                let argc = self.arguments(&call)?;
                                let given: Vec<Instr> = self.piece().instrs.drain(from..).collect();
                                let (Some(Instr::Const(Value::Text(name))), 2) = (given.first(), argc) else {
                                    return Err(format!("{}() needs a quoted name and a value", tok.lexeme));
                                };
                                let name = name.to_string();
                                for w in relocated(given[1..].to_vec(), -1) {
                                    self.put(w);
                                }
                                // A name carrying the scope mark spells
                                // one of a class's own values and no
                                // constant at all. A language with words
                                // for that refuses the name, saying them
                                // where the run reaches the call, so the
                                // program may take the fault as any other.
                                let scoped = self.lang.scope_mark.as_ref().map_or(false, |m| name.contains(m.as_str()));
                                match (scoped, self.lang.define_scoped.clone()) {
                                    (true, Some(said)) => {
                                        self.put_away();
                                        self.constant(Value::text(&said));
                                        self.act(Action::Builtin(Builtin::Raise, tok.lexeme.as_str().into()), 1);
                                    }
                                    _ => {
                                        self.write_global(&name);
                                        self.constant(Value::Flag(true));
                                    }
                                }
                            } else {
                                let argc = self.arguments_of(&tok.lexeme, &call)?;
                                self.call(&tok.lexeme, argc)?;
                            }
                        }
                        // A word standing for all the outermost bindings
                        // taken as an array: a place in it is the binding
                        // whose name that place spells, which is how a
                        // language reaches a global from inside a routine.
                        _ if Lang::spells(&lang.globals_words, &tok.lexeme)
                            && lang.index_brackets.as_ref().map_or(false, |b| self.at_symbol(&b.open)) =>
                        {
                            let index = lang.index_brackets.clone().expect("the index brackets");
                            self.take();
                            self.expr(0)?;
                            self.want_sign(&index.close, "after the name of the binding")?;
                            self.act(Action::Named, 1);
                        }
                        // A name written before the scope mark names a
                        // class, so it is read as one.
                        _ if lang.scope_mark.as_ref().map_or(false, |m| self.at_symbol(m)) => {
                            self.read_class(&tok.lexeme)?;
                        }
                        // A source read in stands where a value does as
                        // well as where a statement does, brackets or
                        // none: `return include $p`. What follows is the
                        // whole of an expression, since nothing written
                        // after it binds more loosely.
                        _ if lang.bare_calls
                            && matches!(lang.builtins.get(&tok.lexeme), Some(Builtin::Include) | Some(Builtin::IncludeOnce)) =>
                        {
                            let named = tok.lexeme.clone();
                            self.expr(0)?;
                            self.call(&named, 1)?;
                        }
                        _ => self.read(&tok.lexeme),
                    }
                }
            }
            Shape::Sign => {
                if let Some(group) = lang.grouping.clone() {
                    if tok.lexeme == group.open {
                        self.take();
                        if self.yield_operand {
                            let mut count = 0;
                            let mut tuple = self.at_symbol(&group.close);
                            while !self.at_symbol(&group.close) {
                                self.expr(0)?;
                                count += 1;
                                if !lang.calling.as_ref().and_then(|c| c.between.as_ref()).map_or(false, |s| self.at_symbol(s)) { break; }
                                tuple = true;
                                self.take();
                            }
                            if tuple { self.act(Action::MakeArray, count); }
                            self.want_sign(&group.close, "to close a group")?;
                        } else {
                        if let Some(clause) = self.comprehension_ahead() {
                            self.comprehension(&group, clause, false)?;
                        } else {
                            if self.at_symbol(&group.close) && !lang.tuple_marks.is_empty() {
                                self.scope_fault(&lang.scope_unready.clone());
                            } else if lang.tuple_marks.is_empty() { self.expr(0)?; }
                            else { self.scope_value()?; }
                            self.want_sign(&group.close, "to close a group")?;
                        }
                        }
                        self.called_on_value()?;
                        return self.indexing(from);
                    }
                }
                if let Some(array) = lang.array_brackets.clone() {
                    if tok.lexeme == array.open {
                        self.take();
                        if !lang.comprehension_for.is_empty() || !lang.array_spread.is_empty() {
                            self.extended_literal(&array, false)?;
                        } else {
                            let count = self.elements(&array)?;
                        self.act(Action::MakeArray, count);
                        }
                        return self.indexing(from);
                    }
                }
                if let Some(map) = lang.map_brackets.clone() {
                    if tok.lexeme == map.open {
                        self.take();
                        if !lang.comprehension_for.is_empty() || !lang.map_spread.is_empty() || lang.set_literals {
                            self.extended_literal(&map, true)?;
                        } else {
                            let count = self.elements(&map)?;
                        self.act(Action::MakeMap, count);
                        }
                        return self.indexing(from);
                    }
                }
                // The words a language puts before whatever stopped the
                // reading are its own; the kernel's plainer ones stand
                // where a definition gives none.
                return Err(match &self.lang.reading_unexpected {
                    Some(opening) => format!("{} {}", opening, tok.lexeme),
                    None => format!("Unexpected token: {}", tok.lexeme),
                });
            }
            _ => return Err("Expected an expression".to_string()),
        }
        self.indexing(from)
    }

    /// The name a class is filed under. Where a language knows a class
    /// by its name however the name is written, every class is filed
    /// with its letters made small, so that each spelling finds it.
    fn class_key(&self, name: &str) -> String {
        // A binding standing where a class is named is a binding still,
        // and bindings are told apart by how they are written; only a
        // class's own name is filed however it is written.
        let a_binding = self.lang.sigil.map_or(false, |mark| name.starts_with(mark));
        if a_binding {
            return name.to_string();
        }
        let folded = match self.lang.classes_folded {
            true => name.to_lowercase(),
            false => name.to_string(),
        };
        format!("{}{}", folded, crate::code::OF_A_CLASS)
    }

    /// What is left of a class named by a value: a property of a thing,
    /// a place in an array, one after another. A call that follows the
    /// chain belongs to the maker and never to the chain, which is why
    /// no step of it is ever a call.
    fn class_reference(&mut self) -> Res<()> {
        let lang = self.lang;
        if let Some(mark) = lang.member_mark.clone() {
            while self.at_symbol(&mark) {
                self.take();
                let member = self.want_name("after the member mark")?;
                self.act(Action::Grab(Rc::from(member.as_str())), 1);
            }
        }
        if let Some(index) = lang.index_brackets.clone() {
            while self.at_symbol(&index.open) {
                self.take();
                self.expr(0)?;
                self.want_sign(&index.close, "after the place naming the class")?;
                self.act(Action::At, 2);
            }
        }
        Ok(())
    }

    /// The class a name stands for: `self` and `parent` name the class
    /// being read and the one it stands on.
    fn read_class(&mut self, name: &str) -> Res<()> {
        let lang = self.lang;
        if Lang::spells(&lang.self_words, name) {
            let (here, _) = self.within.clone().ok_or_else(|| format!("'{}' belongs inside a class", name))?;
            let filed = self.class_key(&here);
            self.read_filed(&here, &filed);
            return Ok(());
        }
        if Lang::spells(&lang.parent_words, name) {
            let (here, base) = self.within.clone().ok_or_else(|| format!("'{}' belongs inside a class", name))?;
            let base = base.ok_or_else(|| format!("Class {} stands on nothing", here))?;
            let filed = self.class_key(&base);
            self.read_filed(&base, &filed);
            return Ok(());
        }
        let named = self.class_key(name);
        self.read_filed(name, &named);
        Ok(())
    }

    /// Reads what a class is filed under, while the name a fault speaks
    /// of stays the one the reader wrote: what a class is filed under is
    /// the kernel's own making and is never anybody else's to read.
    fn read_filed(&mut self, written: &str, filed: &str) {
        let slot = self.cell_to_read(filed, false);
        self.put(Instr::Read(Cell { ident: Rc::from(written), ..slot }));
    }

    /// `unset(a, b[k])`: each name is left as though nothing were ever
    /// written to it, and each place named is taken out of its array.
    fn forget(&mut self, call: &Brackets) -> Res<()> {
        self.forget_targets(call, true, false)
    }

    fn forget_targets(&mut self, call: &Brackets, bracketed: bool, targets: bool) -> Res<()> {
        while !(if bracketed { self.at_symbol(&call.close) } else { self.on_sep() || self.look().shape == Shape::Close || self.exhausted() }) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", call.close));
            }
            if targets {
                let pair = self.lang.grouping.clone().filter(|p| self.at_symbol(&p.open))
                    .or_else(|| self.lang.array_brackets.clone().filter(|p| self.at_symbol(&p.open)));
                if let Some(mut pair) = pair {
                    pair.between = call.between.clone();
                    self.take();
                    self.forget_targets(&pair, true, true)?;
                    self.discard();
                    if call.between.as_ref().map_or(false, |s| self.at_symbol(s)) { self.take(); }
                    continue;
                }
            }
            let from = self.mark();
            self.awkward_place = false;
            if targets { self.block_place()?; } else { self.expr(0)?; }
            let named: Vec<Instr> = self.piece().instrs.drain(from..).collect();
            if self.awkward_place {
                self.constant(Value::text(&self.lang.del_unrun));
                self.act(Action::Builtin(Builtin::Raise, Rc::from("")), 1);
            } else { match named.as_slice() {
                [Instr::Read(slot)] => {
                    if targets { self.put(Instr::Read(slot.clone())); self.discard(); }
                    let held = self.cell_to_write(&slot.ident.to_string());
                    self.put(Instr::Forget(held));
                }
                [.., Instr::Act(Action::At, 2)] if targets => {
                    let (keys, starts) = keys_apart(&named, from, &self.keyed);
                    let Some(last) = keys.len().checked_sub(1) else {
                        return Err("Only a place in a named array has a cell to share".to_string());
                    };
                    let holder = named[..starts[last] - from].to_vec();
                    self.keyed = starts[..last].to_vec();
                    self.footing_cell(&holder, from)?;
                    let at = self.mark();
                    for word in relocated(keys[last].clone(), at as i64 - starts[last] as i64) { self.put(word); }
                    self.act(Action::ForgetWithin, 2);
                    self.discard();
                }
                // `unset($o->p[k])`, `unset(C::$a[k])`: what holds the
                // place is asked for its own cell, and the place taken
                // out of what that cell holds.
                [.., Instr::Act(Action::At, 2)] if self.footed(&named, from) => {
                    let (keys, key_at) = keys_apart(&named, from, &self.keyed);
                    let last = keys.len() - 1;
                    let holder = named[..key_at[last] - from].to_vec();
                    self.footing_cell(&holder, from)?;
                    let at = self.mark();
                    for w in relocated(keys[last].clone(), at as i64 - key_at[last] as i64) {
                        self.put(w);
                    }
                    self.act(Action::ForgetWithin, 2);
                    self.discard();
                }
                [Instr::Read(slot), index @ .., Instr::Act(Action::At, 2)] => {
                    let name = slot.ident.to_string();
                    let at = self.mark();
                    self.read(&name);
                    for w in relocated(index.to_vec(), at as i64 - from as i64) {
                        self.put(w);
                    }
                    self.act(Action::Builtin(Builtin::Erase, Rc::from("unset")), 2);
                    self.write(&name);
                }
                // `unset($o->p)`: the property is taken off the thing
                // itself, which every name for it sees at once.
                [rest @ .., Instr::Act(Action::Grab(member), 1)] => {
                    let (member, rest) = (member.clone(), rest.to_vec());
                    let at = self.mark();
                    for w in relocated(rest, at as i64 - from as i64) {
                        self.put(w);
                    }
                    self.act(Action::Uproot(member), 1);
                    self.discard();
                }
                // `unset($$x)`: the name is worked out as the run goes
                // and the binding it spells left standing for nothing.
                [rest @ .., Instr::Act(Action::Named, 1)] => {
                    let rest = rest.to_vec();
                    let at = self.mark();
                    for w in relocated(rest, at as i64 - from as i64) {
                        self.put(w);
                    }
                    self.act(Action::ForgetNamed, 1);
                    self.discard();
                }
                _ => return Err("Only a name, a place in an array or a property can be forgotten".to_string()),
            } }
            if let Some(sep) = &call.between {
                if self.at_symbol(sep) {
                    self.take();
                }
            }
        }
        if bracketed { self.take(); }
        self.constant(Value::Null);
        Ok(())
    }

    /// The value that spells a member's name: a piece written within
    /// the block marks, or a bare variable — bare, since a call bracket
    /// after it opens the method's arguments and not a call of the
    /// variable itself.
    fn member_value_name(&mut self, member: bool) -> Res<()> {
        let lang = self.lang;
        // After the mark that reaches into a class, the mark is the one
        // a class's own values are written with, and what follows it
        // spells the name outright: `C::$$n` is the value named by what
        // `$n` holds. After the mark that reaches into a thing there is
        // no such mark in the writing, so one standing there says the
        // piece spells a name and the member is named by what *that*
        // binding holds: `$o->${e}` is one step further in.
        if Lang::spells(&lang.naming_words, &self.look().lexeme) {
            self.take();
            self.naming()?;
            if member {
                self.act(Action::Named, 1);
            }
            return Ok(());
        }
        if let (Some(open), Some(close)) = (lang.block_opens.first().cloned(), lang.block_closes.first().cloned()) {
            if self.at_symbol(&open) {
                self.take();
                self.expr(0)?;
                self.want_sign(&close, "after the member's name")?;
                return Ok(());
            }
        }
        let named = self.take().lexeme;
        self.read(&named);
        Ok(())
    }

    /// Whether what stands after the member mark is a value rather than
    /// a name written out: a variable, or a piece written within the
    /// block marks.
    fn member_named_by_value(&mut self, member: bool) -> bool {
        let lang = self.lang;
        if lang.block_opens.first().map_or(false, |open| self.at_symbol(open)) {
            return true;
        }
        if Lang::spells(&lang.naming_words, &self.look().lexeme) {
            return true;
        }
        // A bare variable names a member of a thing by what it holds,
        // but the same written after the mark that reaches into a class
        // names that class's own value outright, mark and all. Only the
        // mark that says a value spells a name works there.
        let here = self.look();
        let a_binding = here.shape == Shape::Instr && lang.sigil.map_or(false, |mark| here.lexeme.starts_with(mark));
        // Unless a call follows it: `C::$m()` calls the method whose
        // name the binding holds, where `C::$m` is the class's own value
        // of that name.
        let calling = lang.calling.as_ref().map_or(false, |c| self.look_ahead(1).is_lexeme(Shape::Sign, &c.open));
        a_binding && (member || calling)
    }

    /// What follows the mark that says a value spells a name: a piece
    /// written within the block marks, or whatever binds as tightly as
    /// a negation, so that `$$$a` reads from the inside out.
    fn naming(&mut self) -> Res<()> {
        let lang = self.lang;
        if let (Some(open), Some(close)) = (lang.block_opens.first().cloned(), lang.block_closes.first().cloned()) {
            if self.at_symbol(&open) {
                self.take();
                self.expr(0)?;
                self.want_sign(&close, "after the name to work out")?;
                return Ok(());
            }
        }
        let tier = lang.monadic.values().map(|m| m.level).max().unwrap_or(0);
        self.expr(tier)
    }

    /// Step past a bracketed piece without reading it, so that what
    /// follows may be read first: the places a taking-apart names are
    /// read only after the value they take from is worked out.
    fn skip_past_call(&mut self) -> Res<()> {
        let call = self.lang.calling.clone().ok_or("A taking-apart needs the call brackets")?;
        self.want_sign(&call.open, "after the word that takes a value apart")?;
        let mut deep = 1usize;
        while deep > 0 {
            if self.exhausted() {
                return Err(format!("Expected '{}'", call.close));
            }
            if self.at_symbol(&call.open) {
                deep += 1;
            } else if self.at_symbol(&call.close) {
                deep -= 1;
            }
            self.take();
        }
        Ok(())
    }

    /// Whether a read of a place in an array stands on something that
    /// has a cell of its own: a property, a binding named as the run
    /// goes, or another such place.
    fn footed(&self, read: &[Instr], from: usize) -> bool {
        let (keys, key_at) = keys_apart(read, from, &self.keyed);
        if keys.is_empty() {
            return false;
        }
        matches!(
            read[..key_at[0] - from],
            [.., Instr::Act(Action::Grab(_) | Action::Reach(_) | Action::Named, 1)]
        ) || matches!(read[..key_at[0] - from], [.., Instr::Act(Action::At, 2)])
    }

    /// The cell of whatever a chain of places stands on, left on the
    /// stack: a name of its own, a property, a binding named as the run
    /// goes, or a place in one of those.
    fn footing_cell(&mut self, read: &[Instr], from: usize) -> Res<()> {
        match read {
            [Instr::Read(slot)] if !slot.moving => {
                let shared = self.cell_to_write(&slot.ident.to_string());
                self.put(Instr::Bond(shared));
            }
            [rest @ .., Instr::Act(Action::Grab(member), 1)] => {
                let (member, rest) = (member.clone(), rest.to_vec());
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.act(Action::BondField(member), 1);
            }
            [rest @ .., Instr::Act(Action::Reach(member), 1)] => {
                let (member, rest) = (member.clone(), rest.to_vec());
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.act(Action::BondOwn(member), 1);
            }
            [rest @ .., Instr::Act(Action::Named, 1)] => {
                let rest = rest.to_vec();
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.act(Action::BondNamed, 1);
            }
            [.., Instr::Act(Action::At, 2)] => {
                let (keys, key_at) = keys_apart(read, from, &self.keyed);
                if keys.is_empty() {
                    return Err("Only a place in a named array has a cell to share".to_string());
                }
                let under = read[..key_at[0] - from].to_vec();
                self.footing_cell(&under, from)?;
                for (i, key) in keys.iter().enumerate() {
                    let at = self.mark();
                    for w in relocated(key.clone(), at as i64 - key_at[i] as i64) {
                        self.put(w);
                    }
                }
                self.put(Instr::Hush(true));
                self.act(Action::BondWithin(keys.len()), keys.len() + 1);
                self.put(Instr::Hush(false));
            }
            _ => return Err("Only a place in a named array has a cell to share".to_string()),
        }
        Ok(())
    }

    /// What stands after the mark that shares a cell: a name, a place in
    /// an array, or a property. Whichever it is, a cell is made of it
    /// where it is not one already and left on the stack, so a name may
    /// be fastened to it.
    fn a_cell(&mut self, unshared: &[String], at_run: bool, handed: Option<(String, usize, String)>) -> Res<()> {
        let from = self.mark();
        // Where the asking stands, so that words said about it name that
        // line and not whatever line a call along the way ran last.
        let row = (self.look().row as u32).saturating_sub(self.before);
        // Read as any expression is, a write among them: what is asked
        // to share a cell may be a write, and a write is not a place, so
        // its value is handed over and the language says so.
        self.expr_at(0, true)?;
        let read: Vec<Instr> = self.piece().instrs.drain(from..).collect();
        match read.as_slice() {
            [Instr::Read(slot)] if !slot.moving => {
                let shared = self.cell_to_write(&slot.ident.to_string());
                self.put(Instr::Bond(shared));
            }
            [rest @ .., Instr::Act(Action::Grab(member), 1)] => {
                let (member, rest) = (member.clone(), rest.to_vec());
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.act(Action::BondField(member), 1);
            }
            // A class's own value is held once for the whole class, and
            // so has a cell as a binding does.
            [rest @ .., Instr::Act(Action::Reach(member), 1)] => {
                let (member, rest) = (member.clone(), rest.to_vec());
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.act(Action::BondOwn(member), 1);
            }
            // A binding named as the run goes has a cell as any binding
            // does, and asking for it is asking for that one.
            [rest @ .., Instr::Act(Action::Named, 1)] => {
                let rest = rest.to_vec();
                let at = self.mark();
                for w in relocated(rest, at as i64 - from as i64) {
                    self.put(w);
                }
                self.act(Action::BondNamed, 1);
            }
            // `&$o->p[k]`: the chain stands on something that is not a
            // binding, so that footing is asked for its own cell first
            // and the place within taken from that.
            [.., Instr::Act(Action::At, 2)] if self.footed(&read, from) => {
                let (keys, key_at) = keys_apart(&read, from, &self.keyed);
                let footing = read[..key_at[0] - from].to_vec();
                self.footing_cell(&footing, from)?;
                for (i, key) in keys.iter().enumerate() {
                    let at = self.mark();
                    for w in relocated(key.clone(), at as i64 - key_at[i] as i64) {
                        self.put(w);
                    }
                }
                self.put(Instr::Hush(true));
                self.act(Action::BondWithin(keys.len()), keys.len() + 1);
                self.put(Instr::Hush(false));
            }
            [Instr::Read(slot), .., Instr::Act(Action::At, 2)] if !slot.moving => {
                let name = slot.ident.to_string();
                // The keys are taken apart so that each is worked out
                // once and in order, and the chain walked by them.
                let (keys, key_at) = keys_apart(&read, from, &self.keyed);
                if keys.is_empty() {
                    return Err("Only a place in a named array has a cell to share".to_string());
                }
                for (i, key) in keys.iter().enumerate() {
                    let at = self.mark();
                    for w in relocated(key.clone(), at as i64 - key_at[i] as i64) {
                        self.put(w);
                    }
                }
                // Asking for a place's cell is a write as much as a
                // read, so a name holding nothing yet is not complained
                // about where the language makes what a write needs.
                let held = self.cell_to_read(&name, false);
                self.put(Instr::Hush(true));
                self.put(Instr::BondPlace(held, keys.len()));
                self.put(Instr::Hush(false));
            }
            // Anything else is read as it stands: a call of a routine
            // that gives back a cell answers with one already, and what
            // has no cell to share is written plainly, which is what a
            // language asking to share one from something that has none
            // does rather than stopping.
            _ => {
                let at = self.mark();
                for w in relocated(read.clone(), at as i64 - from as i64) {
                    self.put(w);
                }
                // A call of a routine that gives back a cell answers
                // with one, so nothing is said of it. Where a language
                // has words for the rest, they are said here, once the
                // value has been worked out.
                let shares = match read.last() {
                    // A method is declared as any other routine is, so
                    // one written to give back a cell gives one however
                    // it is called: through a thing, or through a class.
                    Some(Instr::Act(Action::Invoke(name) | Action::Send(name) | Action::Summon(name), _)) => self.gives_back.contains(name.as_ref()),
                    _ => false,
                };
                // Whether a name may be fastened to what a call answers
                // with is settled by how the routine is written, so it
                // is known here. Whether a routine giving back a cell
                // was given one to give is settled by the run: the value
                // itself says, being a cell or not.
                // A write that fastens one name to another's cell has
                // that very cell to hand over: the write is done and the
                // cell it made is what goes over, not what it holds.
                let fastened = read.iter().rev().find_map(|w| match w {
                    Instr::Fasten(held) => Some(held.clone()),
                    _ => None,
                });
                if let (Some(held), Some(_)) = (fastened, &handed) {
                    self.put_away();
                    self.put(Instr::Bond(held));
                    return Ok(());
                }
                // A call answering with a value where a cell was asked
                // for is a thing a language may only remark upon; a
                // value that was never going to have one — a literal, a
                // write — it refuses outright, naming the parameter.
                // A call answering with a value where a cell was asked
                // for is a thing a language may only remark upon; a
                // value that was never going to have one — a literal, a
                // write — it refuses outright, naming the parameter.
                // Only the run can tell them apart, since a write of one
                // name to another's cell answers with that very cell.
                let calls = matches!(read.last(), Some(Instr::Act(Action::Invoke(_) | Action::Send(_) | Action::Summon(_) | Action::Builtin(..), _)));
                if let Some((called, which, param)) = &handed {
                    if self.lang.tells_place && row > 0 {
                        self.put(Instr::Line(row));
                        self.piece().line = row;
                    }
                    match (calls, unshared.first()) {
                        (true, Some(said)) => self.act(Action::HeldOrSaid(Complaint::Notice, Rc::from(said.as_str())), 1),
                        (true, None) => {}
                        (false, _) => {
                            let told = format!("{}(): Argument #{} ({}) could not be passed by reference", called, which + 1, param);
                            self.act(Action::HeldOrStop(Rc::from(told.as_str())), 1);
                        }
                    }
                    return Ok(());
                }
                let says = at_run || !shares;
                if let (Some(said), true) = (unshared.first(), says) {
                    if self.lang.tells_place && row > 0 {
                        self.put(Instr::Line(row));
                        self.piece().line = row;
                    }
                    let words = Rc::from(said.as_str());
                    match at_run {
                        true => self.act(Action::HeldAnyway(Complaint::Notice, words), 1),
                        false => {
                            self.act(Action::Remark(Complaint::Notice, words), 0);
                            self.put_away();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// `list($a, , $b) = v`: each place named takes the matching place
    /// of the value. A place left out is skipped and still counts, and
    /// a taking-apart within a taking-apart takes the place holding it.
    /// The whole comes to the value, as any other write does.
    fn unpack(&mut self, holding: &str) -> Res<()> {
        let lang = self.lang;
        let call = lang.calling.clone().ok_or("A taking-apart needs the call brackets")?;
        self.want_sign(&call.open, "after the word that takes a value apart")?;
        let mut at = 0usize;
        while !self.at_symbol(&call.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", call.close));
            }
            let sep = call.between.clone();
            let skipped = sep.as_ref().map_or(false, |s| self.at_symbol(s)) || self.at_symbol(&call.close);
            if !skipped {
                // What this place of the value holds, kept aside so the
                // store may read it once.
                let held = self.gensym("place");
                self.read(holding);
                self.constant(Value::Small(at as i64));
                self.act(Action::Apart, 2);
                self.write(&held);
                if Lang::spells(&lang.unpack_words, &self.look().lexeme) {
                    self.take();
                    self.unpack(&held)?;
                    self.discard();
                } else {
                    let from = self.mark();
                    self.expr_at(0, false)?;
                    let was = self.waiting.replace(held);
                    let done = self.store_into(from, None, None, "=");
                    self.waiting = was;
                    done?;
                }
            }
            at += 1;
            if let Some(s) = &sep {
                if self.at_symbol(s) {
                    self.take();
                    continue;
                }
            }
            break;
        }
        self.want_sign(&call.close, "after the places to take apart")?;
        self.read(holding);
        Ok(())
    }

    /// `isset(a, b[k])`: whether every one of them holds something other
    /// than nothing. A binding never written and a place an array does
    /// not hold are both nothing, and neither is complained about, so
    /// every read within is a gentle one.
    /// `empty(x)`: whether what the name or place holds is untrue,
    /// asked as gently as asking whether it is there at all, since a
    /// place that is not there holds nothing and nothing is untrue.
    fn hollow(&mut self, call: &Brackets) -> Res<()> {
        let from = self.mark();
        self.put(Instr::Hush(true));
        self.expr(0)?;
        for w in self.piece().instrs[from..].iter_mut() {
            if let Instr::Act(Action::At, 2) = w {
                *w = Instr::Act(Action::Peek, 2);
            }
        }
        self.put(Instr::Hush(false));
        self.want_sign(&call.close, "after what is asked about")?;
        self.act(Action::AsBool, 1);
        self.act(Action::Not, 1);
        Ok(())
    }

    fn held(&mut self, call: &Brackets) -> Res<()> {
        let mut asked = 0;
        while !self.at_symbol(&call.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", call.close));
            }
            let from = self.mark();
            self.put(Instr::Hush(true));
            self.expr(0)?;
            // Every look inside becomes a gentle one, so that a place
            // that is not there answers nothing instead of stopping.
            for w in self.piece().instrs[from..].iter_mut() {
                if let Instr::Act(Action::At, 2) = w {
                    *w = Instr::Act(Action::Peek, 2);
                }
            }
            self.put(Instr::Hush(false));
            self.act(Action::Nothing, 1);
            self.act(Action::Not, 1);
            asked += 1;
            if let Some(sep) = &call.between {
                if self.at_symbol(sep) {
                    self.take();
                }
            }
        }
        self.take();
        if asked == 0 {
            return Err("Nothing was asked about".to_string());
        }
        for _ in 1..asked {
            self.act(Action::And, 2);
        }
        Ok(())
    }

    /// `expr[i]`, `expr->member` and `expr::member`, repeatable.
    /// A value standing in a group may be called outright: `(f)(x)`,
    /// and one call after another, `(f)(x)(y)`. What is called is held
    /// aside while the arguments are worked out, since a call wants what
    /// it calls above them.
    fn called_on_value(&mut self) -> Res<()> {
        let Some(call) = self.lang.calling.clone() else { return Ok(()) };
        while self.at_symbol(&call.open) {
            let callee = self.gensym("callee");
            self.write(&callee);
            self.take();
            let argc = self.arguments(&call)?;
            self.read(&callee);
            self.act(Action::Invoke(Rc::from("the value the group came to")), argc + 1);
        }
        Ok(())
    }

    /// One place or span within brackets. Commas belong to the row
    /// of places, and are left for the brackets themselves to gather.
    fn slice_part(&mut self, close: &str, separator: Option<&str>) -> Res<()> {
        let marks = self.lang.slice_marks.clone();
        let ellipsis = self.lang.slice_ellipsis.clone();
        if ellipsis.iter().any(|word| self.at_symbol(word)) {
            self.take();
            self.act(Action::SliceUnavailable, 0);
            return Ok(());
        }
        if marks.iter().any(|m| self.at_symbol(m)) {
            self.constant(Value::Null);
        } else {
            self.expr(0)?;
        }
        if !marks.iter().any(|m| self.at_symbol(m)) { return Ok(()); }
        self.take();
        let ended = |reader: &Self| reader.at_symbol(close) || separator.map_or(false, |sep| reader.at_symbol(sep));
        if ended(self) || marks.iter().any(|m| self.at_symbol(m)) {
            self.constant(Value::Null);
        } else {
            self.expr(0)?;
        }
        if marks.iter().any(|m| self.at_symbol(m)) {
            self.take();
            if ended(self) {
                self.constant(Value::Null);
            } else {
                self.expr(0)?;
            }
        } else {
            self.constant(Value::Null);
        }
        self.act(Action::Slice, 3);
        Ok(())
    }

    fn indexing(&mut self, from: usize) -> Res<()> {
        let lang = self.lang;
        if !lang.lambda_words.is_empty() { self.called_on_value()?; }
        loop {
            let member = lang.member_mark.as_ref().map_or(false, |m| self.at_symbol(m));
            let scope = lang.scope_mark.as_ref().map_or(false, |m| self.at_symbol(m));
            if !member && !scope {
                break;
            }
            self.take();
            // A value may stand where a member's name stands: the
            // member is the one that value spells, worked out while the
            // program runs.
            if lang.members_by_value && self.member_named_by_value(member) {
                let names_at = self.mark();
                self.member_value_name(member)?;
                let call = lang.calling.clone().filter(|c| self.at_symbol(&c.open));
                if !member {
                    let Some(call) = call else {
                        // A class's own value, named by what the value
                        // spells.
                        self.act(Action::ReachNamed, 2);
                        continue;
                    };
                    // A method of the class, named by what the value
                    // spells. The name is worked out first and kept
                    // aside; then the object a method is given, what
                    // names the class, and the arguments.
                    let both: Vec<Instr> = self.piece().instrs.drain(from..).collect();
                    let (naming_class, naming_method) = both.split_at(names_at - from);
                    let at = self.mark();
                    for w in relocated(naming_method.to_vec(), at as i64 - names_at as i64) {
                        self.put(w);
                    }
                    let held = self.gensym("called");
                    self.write(&held);
                    match (&lang.this_word, self.within.is_some()) {
                        (Some(this), true) => self.read(&this.clone()),
                        _ => self.constant(Value::Null),
                    }
                    let at = self.mark();
                    for w in relocated(naming_class.to_vec(), at as i64 - from as i64) {
                        self.put(w);
                    }
                    self.take();
                    let argc = self.arguments_of("the method", &call)?;
                    self.read(&held);
                    self.act(Action::SummonNamed(argc), argc + 3);
                    continue;
                }
                match call {
                    Some(call) => {
                        // The name is worked out before the arguments,
                        // and the call wants it on top, so it is kept
                        // aside while they are read.
                        let held = self.gensym("called");
                        self.write(&held);
                        self.take();
                        let argc = self.arguments_of("the method", &call)?;
                        self.read(&held);
                        self.act(Action::SendNamed(argc), argc + 2);
                    }
                    None => self.act(Action::GrabNamed, 2),
                }
                continue;
            }
            let named = self.want_name("after the member mark")?;
            let call = lang.calling.clone().filter(|c| self.at_symbol(&c.open));
            if member {
                match call {
                    Some(call) => {
                        self.take();
                        let argc = self.arguments_of(&named, &call)?;
                        self.act(Action::Send(Rc::from(named.as_str())), argc + 1);
                    }
                    None => self.act(Action::Grab(Rc::from(named.as_str())), 1),
                }
                continue;
            }
            if Lang::spells(&lang.class_words, &named) {
                self.act(Action::Titled, 1);
                continue;
            }
            match call {
                Some(call) => {
                    // The method is given the object it is for, so what
                    // named the class is written again after it.
                    self.take();
                    let named_class: Vec<Instr> = self.piece().instrs.drain(from..).collect();
                    match (&lang.this_word, self.within.is_some()) {
                        (Some(this), true) => self.read(&this.clone()),
                        _ => self.constant(Value::Null),
                    }
                    let at = self.mark();
                    for w in relocated(named_class, at as i64 - from as i64) {
                        self.put(w);
                    }
                    let argc = self.arguments_of(&named, &call)?;
                    self.act(Action::Summon(Rc::from(named.as_str())), argc + 2);
                }
                None => {
                    let bare = lang.sigil.map_or(named.clone(), |s| named.trim_start_matches(s).to_string());
                    self.act(Action::Reach(Rc::from(bare.as_str())), 1);
                }
            }
        }
        let Some(index) = self.lang.index_brackets.clone() else { return Ok(()) };
        let mut stepped = false;
        let mut keyed: Vec<usize> = Vec::new();
        while self.at_symbol(&index.open) {
            // `a[]`: the place after the last, which only a store reaches.
            if self.lang.append_index && self.look_ahead(1).is_lexeme(Shape::Sign, &index.close) {
                self.take();
                self.take();
                self.act(Action::AtEnd, 1);
                continue;
            }
            self.take();
            let began = self.mark();
            let separator = self.lang.calling.as_ref().and_then(|b| b.between.clone());
            self.slice_part(&index.close, separator.as_deref())?;
            if !self.lang.slice_marks.is_empty() && separator.as_ref().map_or(false, |sep| self.at_symbol(sep)) {
                let mut many = 1;
                while separator.as_ref().map_or(false, |sep| self.at_symbol(sep)) {
                    self.take();
                    if self.at_symbol(&index.close) { break; }
                    self.slice_part(&index.close, separator.as_deref())?;
                    many += 1;
                }
                self.act(Action::SliceUnavailable, many);
            }
            self.want_sign(&index.close, "after array index")?;
            keyed.push(began);
            self.act(Action::At, 2);
            stepped = true;
        }
        // A chain read within a key finishes before the chain holding
        // it, so the one left standing is the outermost.
        self.keyed = keyed;
        // What a chain of looks comes to may itself be called: the value
        // is held aside while the arguments are worked out, since a call
        // wants what it calls above them.
        if let Some(call) = self.lang.calling.clone() {
            if stepped && self.at_symbol(&call.open) {
                let callee = self.gensym("callee");
                self.write(&callee);
                self.take();
                let argc = self.arguments(&call)?;
                self.read(&callee);
                self.act(Action::Invoke(Rc::from("the value a look came to")), argc + 1);
                return self.indexing(from);
            }
        }
        // An index may be followed by more members: `$a[0]->b`.
        let more = lang.member_mark.as_ref().map_or(false, |m| self.at_symbol(m))
            || lang.scope_mark.as_ref().map_or(false, |m| self.at_symbol(m));
        if stepped && more {
            return self.indexing(from);
        }
        Ok(())
    }

    /// The elements of a literal: expressions as `arguments` reads them,
    /// except that `k => v` makes one tied value of the two.
    /// Look beyond the first expression, keeping nested brackets whole.
    fn comprehension_ahead(&self) -> Option<usize> {
        if self.lang.comprehension_for.is_empty() { return None; }
        let mut depth = 0usize;
        for (at, tok) in self.tokens.iter().enumerate().skip(self.pos) {
            if !matches!(tok.shape, Shape::Instr | Shape::Sign) { continue; }
            let word = &tok.lexeme;
            if depth == 0 && Lang::spells(&self.lang.comprehension_for, word) {
                return Some(if at > self.pos && Lang::spells(&self.lang.comprehension_async, &self.tokens[at - 1].lexeme) { at - 1 } else { at });
            }
            let opens = [&self.lang.grouping, &self.lang.array_brackets, &self.lang.map_brackets];
            if opens.into_iter().flatten().any(|b| b.open == *word) { depth += 1; }
            else if opens.into_iter().flatten().any(|b| b.close == *word) {
                if depth == 0 { return None; }
                depth -= 1;
            } else if depth == 0 && self.lang.calling.as_ref().and_then(|b| b.between.as_ref()) == Some(word) {
                return None;
            }
        }
        None
    }

    fn literal_is_map(&self, pair: &Brackets) -> bool {
        if self.at_symbol(&pair.close) || self.on_any(&self.lang.map_spread) { return true; }
        let mut depth = 0usize;
        for tok in self.tokens.iter().skip(self.pos) {
            if !matches!(tok.shape, Shape::Instr | Shape::Sign) { continue; }
            if depth == 0 && self.lang.pair_mark.as_ref() == Some(&tok.lexeme) { return true; }
            let brackets = [&self.lang.grouping, &self.lang.array_brackets, &self.lang.map_brackets];
            if brackets.into_iter().flatten().any(|b| b.open == tok.lexeme) { depth += 1; }
            else if brackets.into_iter().flatten().any(|b| b.close == tok.lexeme) {
                if depth == 0 { break; }
                depth -= 1;
            } else if depth == 0 && (pair.between.as_ref() == Some(&tok.lexeme) || Lang::spells(&self.lang.comprehension_for, &tok.lexeme)) { break; }
        }
        false
    }

    /// Each part is worked out before the literal is enlarged by it.
    fn extended_literal(&mut self, pair: &Brackets, braces: bool) -> Res<()> {
        let map = braces && (!self.lang.set_literals || self.literal_is_map(pair));
        if let Some(clause) = self.comprehension_ahead() {
            return self.comprehension(pair, clause, map);
        }
        self.act(if map { Action::MakeMap } else { Action::MakeArray }, 0);
        while !self.at_symbol(&pair.close) {
            let spread = if map { self.on_any(&self.lang.map_spread) } else { self.on_any(&self.lang.array_spread) };
            if spread { self.take(); }
            self.expr(0)?;
            if map && !spread {
                let mark = self.lang.pair_mark.clone().expect("map pair mark");
                self.want_sign(&mark, "between a map key and value")?;
                self.expr(0)?;
                self.act(Action::Tie, 2);
            }
            self.act(Action::GatherItem { map, spread }, 2);
            if self.at_symbol(&pair.close) { break; }
            if let Some(sep) = &pair.between { self.want_sign(sep, "between literal items")?; }
            else { return Err("Expected a literal separator".to_string()); }
        }
        self.want_sign(&pair.close, "after a literal")
    }

    fn comprehension(&mut self, pair: &Brackets, clause: usize, map: bool) -> Res<()> {
        let head = self.pos;
        let bindings = self.comprehension_names.len();
        let result = self.gensym("comprehension");
        self.act(if map { Action::MakeMap } else { Action::MakeArray }, 0);
        self.write(&result);
        self.pos = clause;
        self.comprehension_clause(head, &result, map)?;
        self.comprehension_names.truncate(bindings);
        self.want_sign(&pair.close, "after a comprehension")?;
        self.read(&result);
        Ok(())
    }

    /// The head is read last, once all its names have their own cells.
    fn comprehension_target(&mut self) -> Res<Option<String>> {
        let written = self.look().lexeme.clone();
        let from = self.mark();
        self.prefix()?;
        let target: Vec<Instr> = self.piece().instrs.drain(from..).collect();
        match target.as_slice() {
            [Instr::Read(_)] => Ok(Some(written)),
            [.., Instr::Act(Action::At, 2)] => Ok(None),
            _ => Err("Expected a name or indexed place as a comprehension target".to_string()),
        }
    }

    fn comprehension_clause(&mut self, head: usize, result: &str, map: bool) -> Res<()> {
        if self.on_any(&self.lang.comprehension_async) {
            self.take();
            if !self.on_any(&self.lang.comprehension_for) { return Err("Expected a walk after the asynchronous word".to_string()); }
            let said = self.lang.comprehension_async_unavailable.first().cloned().unwrap_or_else(|| "Asynchronous walks are not provided".to_string());
            self.constant(Value::text(&said));
            self.act(Action::Builtin(Builtin::Raise, Rc::from("comprehension")), 1);
        }
        if self.on_any(&self.lang.comprehension_for) {
            self.take();
            let group = self.lang.grouping.clone();
            let grouped = group.as_ref().map_or(false, |g| self.at_symbol(&g.open));
            if grouped { self.take(); }
            let mut names = vec![self.comprehension_target()?];
            let separator = self.lang.calling.as_ref().and_then(|c| c.between.clone());
            let mut unpack = false;
            while separator.as_ref().map_or(false, |s| self.at_symbol(s)) {
                self.take();
                unpack = true;
                if self.on_any(&self.lang.comprehension_in) || group.as_ref().map_or(false, |g| self.at_symbol(&g.close)) { break; }
                names.push(self.comprehension_target()?);
            }
            if grouped { self.want_sign(&group.expect("target group").close, "after comprehension names")?; }
            if !self.on_any(&self.lang.comprehension_in) { return Err("Expected the comprehension's collection word".to_string()); }
            self.take();
            if names.iter().any(Option::is_none) {
                let said = self.lang.comprehension_target_unavailable.first().cloned().unwrap_or_else(|| "Indexed comprehension targets are not provided".to_string());
                self.constant(Value::text(&said));
                self.act(Action::Builtin(Builtin::Raise, Rc::from("comprehension")), 1);
            }
            self.expr(1)?;
            self.act(Action::ComprehensionItems, 1);
            let bag = self.gensym("comprehension_source");
            self.write(&bag);
            let at = self.gensym("comprehension_place");
            self.constant(Value::Small(0));
            self.write(&at);
            let test = self.mark();
            self.read(&at);
            self.read(&bag);
            self.act(Action::Extent, 1);
            self.act(Action::Lt, 2);
            let done = self.skip();
            self.read(&bag);
            self.read(&at);
            self.act(Action::At, 2);
            let item = self.gensym("comprehension_item");
            if unpack { self.act(Action::UnpackCount(names.len()), 1); }
            self.write(&item);
            for (i, name) in names.into_iter().enumerate() {
                let own = self.gensym("comprehension_name");
                self.read(&item);
                if unpack {
                    self.constant(Value::Small(i as i64));
                    self.act(Action::At, 2);
                }
                self.write(&own);
                if let Some(name) = name { self.comprehension_names.push((name, own)); }
            }
            self.comprehension_clause(head, result, map)?;
            self.read(&at);
            self.constant(Value::Small(1));
            self.act(Action::Add, 2);
            self.write(&at);
            self.constant(Value::Flag(false));
            self.put(Instr::Skip(test));
            self.land(done);
        } else if self.on_any(&self.lang.comprehension_if) {
            self.take();
            self.expr(1)?;
            let rejected = self.skip();
            self.comprehension_clause(head, result, map)?;
            self.land(rejected);
        } else {
            let tail = self.pos;
            self.pos = head;
            self.read(result);
            let spread = self.on_any(if map { &self.lang.map_spread } else { &self.lang.array_spread });
            if spread { self.take(); }
            self.expr(0)?;
            if map && !spread {
                let mark = self.lang.pair_mark.clone().expect("map pair mark");
                self.want_sign(&mark, "between a comprehension key and value")?;
                self.expr(0)?;
                self.act(Action::Tie, 2);
            }
            if !self.on_any(&self.lang.comprehension_for) && !self.on_any(&self.lang.comprehension_async) { return Err("Expected a comprehension clause after its expression".to_string()); }
            self.act(Action::GatherItem { map, spread }, 2);
            self.write(result);
            self.pos = tail;
        }
        Ok(())
    }

    fn elements(&mut self, pair: &Brackets) -> Res<usize> {
        let mark = self.lang.pair_mark.clone();
        let mut count = 0;
        while !self.at_symbol(&pair.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", pair.close));
            }
            // `[&$a]`: the place holds the binding's own cell, so a
            // write through either is a write both see.
            let shared = self.lang.reference_mark.clone().filter(|m| self.at_symbol(m)).is_some();
            if shared {
                self.take();
                let named = self.want_name("as the name to share a cell with")?;
                let cell = self.cell_to_write(&named);
                self.put(Instr::Bond(cell));
            } else {
                self.expr(0)?;
            }
            if let Some(mark) = &mark {
                if self.at_symbol(mark) {
                    self.take();
                    self.expr(0)?;
                    self.act(Action::Tie, 2);
                }
            }
            count += 1;
            if let Some(sep) = &pair.between {
                if self.at_symbol(sep) {
                    self.take();
                }
            }
        }
        self.take();
        Ok(count)
    }

    /// The arguments of a call by name: a parameter written with the
    /// reference sign is given the name's own cell, so what the program
    /// writes to it the caller sees.
    fn arguments_of(&mut self, called: &str, pair: &Brackets) -> Res<usize> {
        let shared = self.shared_args.get(called).cloned().unwrap_or_default();
        if !shared.iter().any(|x| *x) {
            return self.arguments(pair);
        }
        let mut count = 0;
        while !self.at_symbol(&pair.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", pair.close));
            }
            // A parameter that takes a cell is handed the cell of
            // whatever names one: a binding, a place in an array, a
            // property. What has none is handed its value, and the
            // language says so where it has words for it.
            if shared.get(count).copied().unwrap_or(false) {
                let param = self.arg_names.get(called).and_then(|all| all.get(count)).cloned().unwrap_or_default();
                self.a_cell(&self.lang.unshared_handed.clone(), false, Some((called.to_string(), count, param)))?;
            } else {
                self.expr(0)?;
            }
            count += 1;
            if let Some(sep) = &pair.between {
                if self.at_symbol(sep) {
                    self.take();
                }
            }
        }
        self.take();
        Ok(count)
    }

    /// Expressions up to the closing bracket, consumed; a tag before an
    /// argument is dropped. Returns how many.
    fn arguments(&mut self, pair: &Brackets) -> Res<usize> {
        if let Some(clause) = self.comprehension_ahead() {
            self.comprehension(pair, clause, false)?;
            return Ok(1);
        }
        let mut count = 0;
        let mut pieces: Vec<(bool, Vec<Instr>)> = Vec::new();
        while !self.at_symbol(&pair.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", pair.close));
            }
            let start = self.mark();
            let labelled = self.look().shape == Shape::Instr
                && self.look_ahead(1).shape == Shape::Sign
                && (Lang::spells(&self.lang.argument_labels, &self.look_ahead(1).lexeme)
                    || (self.lang.bind_names && Lang::spells(&self.lang.assign_words, &self.look_ahead(1).lexeme)));
            let tagged = self.lang.bind_names && labelled;
            let named_spread = Lang::spells(&self.lang.call_spread_pairs, &self.look().lexeme);
            let spread = self.lang.bind_names && !labelled
                && (Lang::spells(&self.lang.call_spread, &self.look().lexeme)
                    || Lang::spells(&self.lang.call_spread_pairs, &self.look().lexeme));
            if tagged {
                self.constant(Value::text(&self.look().lexeme));
            } else if spread {
                self.constant(Value::Flag(Lang::spells(&self.lang.call_spread_pairs, &self.look().lexeme)));
                self.take();
            }
            if labelled { self.pos += 2; }
            self.expr(0)?;
            if tagged || spread { self.act(Action::Tie, 2); }
            if self.lang.bind_names {
                let code = self.piece().instrs.drain(start..).collect();
                pieces.push((tagged || named_spread, relocated(code, -(start as i64))));
            }
            count += 1;
            if let Some(sep) = &pair.between {
                if self.at_symbol(sep) {
                    self.take();
                }
            }
        }
        self.take();
        // Positional spreads run before named values even when a named
        // argument was written before a spread.
        pieces.sort_by_key(|(named, _)| *named);
        for (_, code) in pieces {
            let start = self.mark();
            self.piece().instrs.extend(relocated(code, start as i64));
        }
        Ok(count)
    }

    /// A call by name, the arguments already on the stack: a builtin of
    /// the definition, or the program bound to the name.
    fn call(&mut self, name: &str, argc: usize) -> Res<()> {
        match self.lang.builtins.get(name).copied() {
            Some(Builtin::Append) | Some(Builtin::Replace) | Some(Builtin::Lead) => {
                Err(format!("First argument to {}() must be an array variable name", name))
            }
            Some(Builtin::Pack) => Err(format!("{}() is a literal, not a call", name)),
            Some(native) => {
                self.act(Action::Builtin(native, Rc::from(name)), argc);
                Ok(())
            }
            None => {
                self.read_callee(name);
                self.act(Action::Invoke(Rc::from(name)), argc + 1);
                Ok(())
            }
        }
    }

    /// push and put write the named array in place: the array is taken
    /// out of its slot, rewritten, and put back; the value is null.
    fn mutation(&mut self, name: &str, target: &str, argc: usize) -> Res<()> {
        let native = self.lang.builtins[name];
        let wanted = if native == Builtin::Append { 2 } else { 3 };
        if argc != wanted && !self.lang.bind_names {
            return Err(format!("{}() expects {} arguments, got {}", name, wanted, argc));
        }
        self.read_taking(target);
        self.act(Action::Builtin(native, Rc::from(name)), argc);
        self.restore(target);
        self.constant(Value::Null);
        Ok(())
    }

    /// Putting values at the head of a named array: the values first,
    /// then the array itself, taken out of its slot and put back with
    /// them in front. How many places it holds afterwards is the answer,
    /// so the array is read once more for its extent.
    fn leading(&mut self, name: &str, target: &str, argc: usize) -> Res<()> {
        if argc == 0 {
            return Err(format!("{}() expects an array and a value at least", name));
        }
        self.read_taking(target);
        self.act(Action::Builtin(Builtin::Lead, Rc::from(name)), argc + 1);
        self.restore(target);
        self.read(target);
        self.act(Action::Extent, 1);
        Ok(())
    }

    // ---------- postfix ----------

    /// Words up to one of `stops` or the end. A quoted name is data for
    /// the word after it. In a block, a deeper indentation is an error and
    /// the block's own end stops the instrs; in a program value the
    /// indentation is free, since its brackets say where it ends; on a
    /// line, the line end stops them.
    fn rpn_body(&mut self, stops: &[String], body: Span) -> Res<()> {
        loop {
            if body == Span::Line {
                while self.look().shape == Shape::Sign && self.lang.ends_stmt(&self.look().lexeme) {
                    self.take();
                }
            } else {
                self.skip_seps();
            }
            if self.exhausted() || self.on_any(stops) {
                return Ok(());
            }
            match (self.look().shape, body) {
                (Shape::LineEnd, Span::Line) | (Shape::Close, Span::Block) | (Shape::Open | Shape::Close, Span::Line) => return Ok(()),
                (Shape::Open | Shape::Close, Span::Routine) => {
                    self.take();
                    continue;
                }
                (Shape::Open, Span::Block) => return Err("Unexpected indentation".to_string()),
                _ => {}
            }
            let tok = self.take();
            match tok.shape {
                Shape::Quoted => {
                    self.skip_seps();
                    let taker = self.take();
                    if !matches!(taker.shape, Shape::Instr | Shape::Sign) {
                        return Err(format!("The name '{}' must be followed by a word that takes it", tok.lexeme));
                    }
                    self.named_word(&taker.lexeme, &tok.lexeme)?;
                }
                Shape::Numeral => {
                    let v = parse_number(&tok.lexeme, self.lang)?;
                    self.constant(v);
                }
                Shape::Quote => self.constant(Value::text(&tok.lexeme)),
                Shape::Instr | Shape::Sign => self.postfix_word(&tok)?,
                _ => unreachable!("separators and block marks are handled above"),
            }
        }
    }

    /// The body after a control word, in the language's block style.
    fn rpn_block(&mut self) -> Res<()> {
        let stops = self.postfix_open()?;
        self.rpn_body(&stops, Span::Block)?;
        self.rpn_close(&stops)
    }

    /// Where a block begins: the instrs that end its body.
    fn postfix_open(&mut self) -> Res<Vec<String>> {
        match self.lang.blocks {
            Blocks::Indented => {
                self.skip_seps();
                if self.look().shape != Shape::Open {
                    return Err(format!("Expected an indented block, got '{}'", self.look().lexeme));
                }
                self.take();
                Ok(Vec::new())
            }
            Blocks::Braced => {
                let which = self.lang.block_opens.iter().position(|o| self.at_lexeme(o));
                let Some(i) = which else {
                    return Err(format!("Expected '{}' to open a block, got '{}'", self.lang.block_opens[0], self.look().lexeme));
                };
                self.take();
                Ok(vec![self.lang.block_closes[i].clone()])
            }
            Blocks::Worded => Ok(self.lang.block_closes.clone()),
        }
    }

    fn rpn_close(&mut self, stops: &[String]) -> Res<()> {
        match self.lang.blocks {
            Blocks::Indented => {
                if self.look().shape != Shape::Close {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.take();
                Ok(())
            }
            Blocks::Braced => self.want_lexeme(&stops[0]),
            Blocks::Worded => self.expect_closer(),
        }
    }

    /// A loop's condition: the instrs up to where its body begins, which
    /// is the intro word, the line end, or the block opener by style.
    fn rpn_test(&mut self) -> Res<()> {
        match self.lang.blocks {
            Blocks::Indented => self.rpn_body(&[], Span::Line),
            Blocks::Braced => {
                let openers = self.lang.block_opens.clone();
                self.rpn_body(&openers, Span::Block)
            }
            Blocks::Worded => {
                let intros = self.lang.block_intros.clone();
                self.rpn_body(&intros, Span::Block)?;
                self.expect_intro()
            }
        }
    }

    /// How far ahead an else word follows, past line ends.
    fn else_ahead(&self) -> Option<usize> {
        let mut ahead = 0;
        loop {
            let t = self.look_ahead(ahead);
            if t.shape == Shape::LineEnd || (t.shape == Shape::Sign && self.lang.ends_stmt(&t.lexeme)) {
                ahead += 1;
            } else {
                return (t.shape == Shape::Instr && Lang::spells(&self.lang.else_words, &t.lexeme)).then_some(ahead);
            }
        }
    }

    /// The word after a quoted name.
    fn named_word(&mut self, word: &str, name: &str) -> Res<()> {
        let lang = self.lang;
        if Lang::spells(&lang.assign_words, word) || Lang::spells(&lang.let_words, word) {
            self.write(name);
            return Ok(());
        }
        if Lang::spells(&lang.for_words, word) {
            let bound = self.gensym("end");
            self.write(&bound);
            self.write(name);
            return self.count_loop(name, &bound, true);
        }
        match lang.builtins.get(word).copied() {
            Some(native @ (Builtin::Append | Builtin::Replace)) => {
                self.read_taking(name);
                self.act(Action::Builtin(native, Rc::from(word)), if native == Builtin::Append { 2 } else { 3 });
                self.restore(name);
                Ok(())
            }
            _ => Err(format!("'{}' does not take a name, but '{}' was given", word, name)),
        }
    }

    /// The RPL stack instrs, as stores and loads of scratch slots.
    fn stack_op(&mut self, word: &str) -> bool {
        let lang = self.lang;
        let (a, b, c) = (SPARE_CELLS[0], SPARE_CELLS[1], SPARE_CELLS[2]);
        let (pops, pushes): (&[&str], &[&str]) = if Lang::spells(&lang.dup_words, word) {
            (&[a], &[a, a])
        } else if Lang::spells(&lang.drop_words, word) {
            (&[a], &[])
        } else if Lang::spells(&lang.swap_words, word) {
            (&[a, b], &[a, b])
        } else if Lang::spells(&lang.over_words, word) {
            (&[a, b], &[b, a, b])
        } else if Lang::spells(&lang.rot_words, word) {
            (&[a, b, c], &[b, a, c])
        } else {
            return false;
        };
        for name in pops {
            self.write(name);
        }
        for name in pushes {
            self.read(name);
        }
        true
    }

    fn postfix_word(&mut self, tok: &Token) -> Res<()> {
        let lang = self.lang;
        let word = tok.lexeme.as_str();
        if Lang::spells(&lang.true_words, word) {
            self.constant(Value::Flag(true));
            return Ok(());
        }
        if Lang::spells(&lang.false_words, word) {
            self.constant(Value::Flag(false));
            return Ok(());
        }
        if Lang::spells(&lang.null_words, word) {
            self.constant(Value::Null);
            return Ok(());
        }
        if Lang::spells(&lang.if_words, word) {
            // In the keyword style one closer ends the whole if/else; in
            // the others each arm is a block of its own.
            let keyword = lang.blocks == Blocks::Worded;
            let skip = self.skip();
            let stops = self.postfix_open()?;
            let mut arm_stops = stops.clone();
            if keyword {
                arm_stops.extend(lang.else_words.iter().cloned());
            }
            self.rpn_body(&arm_stops, Span::Block)?;
            if !keyword {
                self.rpn_close(&stops)?;
            }
            let else_at = if keyword { self.on_any(&lang.else_words).then_some(0) } else { self.else_ahead() };
            match else_at {
                Some(ahead) => {
                    self.pos += ahead + 1;
                    let over = self.leap();
                    self.land(skip);
                    if keyword {
                        self.rpn_body(&stops, Span::Block)?;
                    } else {
                        self.rpn_block()?;
                    }
                    self.land(over);
                }
                None => self.land(skip),
            }
            if keyword {
                self.rpn_close(&stops)?;
            }
            return Ok(());
        }
        if Lang::spells(&lang.while_words, word) {
            let cond_at = self.pos;
            let mark = self.mark();
            self.rpn_test()?;
            let body_at = self.pos;
            self.piece().instrs.truncate(mark);
            let to_test = self.leap();
            let top = self.mark();
            self.enter_cycle(None);
            self.pos = body_at;
            self.rpn_block()?;
            let after = self.pos;
            let test = self.mark();
            self.land(to_test);
            self.pos = cond_at;
            self.rpn_test()?;
            self.pos = after;
            self.loop_back(top);
            self.leave_cycle(test);
            return Ok(());
        }
        if Lang::spells(&lang.until_words, word) {
            // Written first, tested after each pass: lifted past the body.
            let from = self.mark();
            self.rpn_test()?;
            let test: Vec<Instr> = self.piece().instrs.drain(from..).collect();
            let top = self.mark();
            self.enter_cycle(None);
            self.rpn_block()?;
            let again = self.mark();
            for w in relocated(test, again as i64 - from as i64) {
                self.put(w);
            }
            self.put(Instr::Skip(top));
            self.leave_cycle(again);
            return Ok(());
        }
        if Lang::spells(&lang.return_words, word) {
            self.escape();
            return Ok(());
        }
        if Lang::spells(&lang.break_words, word) {
            return self.leave(1);
        }
        if Lang::spells(&lang.continue_words, word) {
            return self.resume(1);
        }
        if Lang::spells(&lang.for_words, word) {
            return Err(format!("'{}' needs a quoted name before it", word));
        }
        if let Some(i) = lang.quote_open.iter().position(|o| o == word) {
            let close = lang.quote_close[i].clone();
            let name = self.gensym("program");
            let program = self.routine(&format!("<{}>", &name[1..]), Vec::new(), 0, false, |a| {
                a.rpn_body(std::slice::from_ref(&close), Span::Routine)?;
                a.want_lexeme(&close)
            })?;
            self.constant(Value::Routine(program));
            return Ok(());
        }
        if let Some(array) = lang.array_brackets.as_ref().filter(|a| a.open == word) {
            let close = array.close.clone();
            self.constant(Value::Fence);
            self.rpn_body(std::slice::from_ref(&close), Span::Block)?;
            self.want_lexeme(&close)?;
            self.act(Action::Collect, 0);
            return Ok(());
        }
        if self.stack_op(word) {
            return Ok(());
        }
        if Lang::spells(&lang.eval_words, word) {
            self.act(Action::Evaluate, 1);
            return Ok(());
        }
        if let Some(infix) = lang.dyadic.get(word) {
            self.act(infix.action.clone(), 2);
            return Ok(());
        }
        if let Some(infix) = lang.monadic.get(word) {
            self.act(infix.action.clone(), 1);
            return Ok(());
        }
        if let Some(native) = lang.builtins.get(word).copied() {
            let (argc, returns_value) = match native {
                Builtin::Append | Builtin::Replace => return Err(format!("'{}' needs a quoted name before it", word)),
                Builtin::External | Builtin::Span => return Err(format!("'{}' has no postfix form", word)),
                Builtin::Echo | Builtin::Say | Builtin::Out | Builtin::Tell | Builtin::Dump | Builtin::Raise => (1, false),
                Builtin::Layout => (1, true),
                Builtin::Define | Builtin::Pack | Builtin::Erase => return Err(format!("'{}' has no postfix form", word)),
                Builtin::CharAtIndex | Builtin::Fetch | Builtin::MakeReal => (2, true),
                _ => (1, true),
            };
            self.act(Action::Builtin(native, Rc::from(word)), argc);
            if !returns_value {
                self.discard();
            }
            return Ok(());
        }
        if tok.shape != Shape::Instr {
            return Err(format!("Unexpected '{}'", word));
        }
        self.read(word);
        self.act(Action::Execute, 1);
        Ok(())
    }
}


fn same_cell(a: &Cell, b: &Cell) -> bool {
    a.ident == b.ident && a.near == b.near && a.far == b.far && !a.moving && !b.moving
}

fn operand_of(w: &Instr) -> Option<Operand> {
    match w {
        Instr::Read(s) if !s.moving => Some(Operand::Cell(s.clone())),
        Instr::Const(v) => Some(Operand::Const(v.clone())),
        _ => None,
    }
}

/// The peephole: where four or three or two of the five instrs spell one
/// of the three fused ones, that one replaces them. Jump targets move
/// with the instrs; a group is left alone if a jump lands inside it.
fn peephole(instrs: Vec<Instr>) -> Vec<Instr> {
    let mut targets = vec![false; instrs.len() + 1];
    for w in &instrs {
        if let Instr::Skip(t) = w {
            targets[*t] = true;
        }
    }
    // Words standing under a guard are left as they are. A fault is
    // offered to the guard where it comes of applying an operation, so
    // fusing the operation into its operands would put it out of reach
    // of the very statement written to take it.
    let mut watched = vec![false; instrs.len()];
    let mut depth = 0usize;
    let mut watched_to = 0;
    for (at, w) in instrs.iter().enumerate() {
        if let Instr::Attempt(plan) = w { watched_to = watched_to.max(plan.after); }
        match w {
            Instr::Guard(_) => depth += 1,
            Instr::Unguard => depth = depth.saturating_sub(1),
            _ => {}
        }
        watched[at] = depth > 0 || at < watched_to;
    }
    let comparison = |op: &Action| matches!(op, Action::Eq | Action::Ne | Action::Lt | Action::Le | Action::Gt | Action::Ge);
    let arithmetic = |op: &Action| comparison(op) || matches!(op, Action::Add | Action::Sub | Action::Mul | Action::Div | Action::DivReal | Action::IntDiv | Action::Mod | Action::Power | Action::Join | Action::At);
    let mut out: Vec<Instr> = Vec::with_capacity(instrs.len());
    let mut map = vec![0usize; instrs.len() + 1];
    let mut i = 0;
    while i < instrs.len() {
        let clear = |width: usize| {
            i + width <= instrs.len() && !targets[i + 1..i + width].iter().any(|&t| t) && !watched[i..i + width].iter().any(|&w| w)
        };
        let mut group = None;
        let mut width = 4;
        if clear(4) {
            group = match (&instrs[i], &instrs[i + 1], &instrs[i + 2], &instrs[i + 3]) {
                (Instr::Read(a), b, Instr::Act(op, 2), Instr::Skip(to)) if !a.moving && comparison(op) => {
                    operand_of(b).map(|b| Instr::SkipCmp { op: op.clone(), a: Operand::Cell(a.clone()), b, to: *to })
                }
                (Instr::Read(a), Instr::Const(k @ Value::Small(_)), Instr::Act(Action::Add, 2), Instr::Write(s)) if same_cell(a, s) => {
                    Some(Instr::Bump { slot: s.clone(), by: k.clone() })
                }
                _ => None,
            };
        }
        if group.is_none() && clear(3) {
            if let (Some(a), Some(b), Instr::Act(op, 2)) = (operand_of(&instrs[i]), operand_of(&instrs[i + 1]), &instrs[i + 2]) {
                if arithmetic(op) {
                    group = Some(Instr::Dyad { op: op.clone(), a, b });
                    width = 3;
                }
            }
        }
        if group.is_none() && clear(2) {
            if let (Some(b), Instr::Act(op, 2)) = (operand_of(&instrs[i]), &instrs[i + 1]) {
                if arithmetic(op) {
                    group = Some(Instr::Dyad { op: op.clone(), a: Operand::Top, b });
                    width = 2;
                }
            }
        }
        // A word that does nothing is taken out here, where the map
        // that follows moves every jump that pointed at it onto the
        // word that took its place.
        if matches!(instrs[i], Instr::Nothing) {
            map[i] = out.len();
            i += 1;
            continue;
        }
        match group {
            Some(w) => {
                for j in i..i + width {
                    map[j] = out.len();
                }
                out.push(w);
                i += width;
            }
            None => {
                map[i] = out.len();
                out.push(instrs[i].clone());
                i += 1;
            }
        }
    }
    map[instrs.len()] = out.len();
    for w in out.iter_mut() {
        match w {
            Instr::Skip(t) | Instr::SkipCmp { to: t, .. } | Instr::Guard(t) => *t = map[*t],
            Instr::Attempt(plan) => plan.move_marks(|i| map[i]),
            Instr::Depart { to, cycle } => {
                *to = map[*to];
                if let Some(start) = cycle { *start = map[*start]; }
            }
            _ => {}
        }
    }
    out
}

/// Whether a word keeps to itself: it works on the stack and on cells
/// and calls nothing, so nothing within it can ask the run anything or
/// take room that is not let go again straight away. An operation is
/// counted as calling out unless it is plain arithmetic over numbers,
/// since joining text and reaching into a value are both ways a class
/// of the program's own can be called without saying so.
fn word_alone(w: &Instr) -> bool {
    let quiet = |op: &Action| {
        matches!(op, Action::Add | Action::Sub | Action::Mul | Action::Div | Action::DivReal | Action::IntDiv
            | Action::Mod | Action::Power | Action::Eq | Action::Ne | Action::Lt | Action::Le | Action::Gt
            | Action::Ge | Action::And | Action::Or | Action::Not | Action::Negate | Action::AsBool
            | Action::Same | Action::Unsame | Action::Rank | Action::Extent | Action::Nothing
            | Action::BitBoth | Action::BitEither | Action::BitOne | Action::BitTurn | Action::BitUp | Action::BitDown)
    };
    match w {
        Instr::Const(_) | Instr::Read(_) | Instr::Glance(_) | Instr::Write(_) | Instr::Skip(_)
        | Instr::Missing(_) | Instr::Unwritten(_) | Instr::Line(_) | Instr::Forget(_)
        | Instr::Emptied(_) | Instr::Shed | Instr::Nothing | Instr::Ready(_) | Instr::Bump { .. } => true,
        Instr::Act(op, _) => quiet(op),
        Instr::Dyad { op, .. } | Instr::SkipCmp { op, .. } => quiet(op),
        _ => false,
    }
}

/// Words moved by `delta`, their jump targets moved with them.
/// Which parameters of each program are written with the reference sign,
/// found by reading the tokens before anything is compiled: a call has to
/// know before it works out its arguments, and a program may be called
/// above where it is written.
fn shared_parameters(tokens: &[Token], lang: &Lang) -> (HashMap<String, Vec<bool>>, HashMap<String, Vec<String>>, std::collections::HashSet<String>) {
    let mut found = HashMap::new();
    let mut called = HashMap::new();
    let mut gives = std::collections::HashSet::new();
    let (Some(mark), Some(call)) = (&lang.reference_mark, &lang.calling) else { return (found, called, gives) };
    let sign = |t: &Token, text: &str| t.is_lexeme(Shape::Sign, text);
    let mut i = 0;
    while i + 2 < tokens.len() {
        let t = &tokens[i];
        if t.shape != Shape::Instr || !Lang::spells(&lang.function_words, &t.lexeme) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let hands_back = sign(&tokens[j], mark);
        if hands_back {
            j += 1;
        }
        if tokens[j].shape != Shape::Instr || !sign(&tokens[j + 1], &call.open) {
            i += 1;
            continue;
        }
        let name = tokens[j].lexeme.clone();
        if hands_back {
            gives.insert(name.clone());
        }
        j += 2;
        let (mut marks, mut shared, mut anything, mut defaulting, mut depth) = (Vec::new(), false, false, false, 1usize);
        let (mut names, mut naming) = (Vec::new(), String::new());
        while j < tokens.len() {
            let p = &tokens[j];
            if sign(p, &call.open) {
                depth += 1;
            } else if sign(p, &call.close) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            } else if depth == 1 {
                anything = true;
                if call.between.as_ref().map_or(false, |s| sign(p, s)) {
                    marks.push(shared);
                    names.push(std::mem::take(&mut naming));
                    (shared, defaulting) = (false, false);
                } else if p.shape == Shape::Sign && Lang::spells(&lang.assign_words, &p.lexeme) {
                    defaulting = true;
                } else if !defaulting && sign(p, mark) {
                    shared = true;
                } else if !defaulting && naming.is_empty() && p.shape == Shape::Instr {
                    naming = p.lexeme.clone();
                }
            }
            j += 1;
        }
        if anything {
            marks.push(shared);
            names.push(naming);
        }
        called.insert(name.clone(), names);
        found.insert(name, marks);
        i = j;
    }
    (found, called, gives)
}

/// The keys of an index chain, each with where it began, taken out of
/// the instrs that read `a[k1][k2]...`. Nothing is given back unless the
/// chain stands on a bare name and every key is followed by the one look
/// that reads it, since only then may a store take the chain apart.
fn keys_apart(target: &[Instr], from: usize, keyed: &[usize]) -> (Vec<Vec<Instr>>, Vec<usize>) {
    let nothing = (Vec::new(), Vec::new());
    let appending = matches!(target.last(), Some(Instr::Act(Action::AtEnd, 1)));
    let reach = target.len() - usize::from(appending);
    if keyed.is_empty() || reach == 0 || !matches!(target[reach - 1], Instr::Act(Action::At, 2)) {
        return nothing;
    }
    // The chain stands on whatever comes before its first key, which
    // may be a name, a property, a class's own value or anything else
    // that can be read and written.
    if keyed[0] <= from || keyed.windows(2).any(|w| w[0] >= w[1]) || keyed[keyed.len() - 1] >= from + reach {
        return nothing;
    }
    let mut keys = Vec::new();
    for i in 0..keyed.len() {
        let began = keyed[i] - from;
        let ended = match keyed.get(i + 1) {
            Some(next) => next - from - 1,
            None => reach - 1,
        };
        if began >= ended || !matches!(target[ended], Instr::Act(Action::At, 2)) {
            return nothing;
        }
        keys.push(target[began..ended].to_vec());
    }
    (keys, keyed.to_vec())
}

fn relocated(instrs: Vec<Instr>, delta: i64) -> Vec<Instr> {
    instrs
        .into_iter()
        .map(|w| match w {
            Instr::Skip(t) => Instr::Skip((t as i64 + delta) as usize),
            Instr::Guard(t) => Instr::Guard((t as i64 + delta) as usize),
            Instr::Depart { to, cycle } => Instr::Depart {
                to: (to as i64 + delta) as usize,
                cycle: cycle.map(|i| (i as i64 + delta) as usize),
            },
            Instr::Attempt(mut plan) => {
                plan.move_marks(|i| (i as i64 + delta) as usize);
                Instr::Attempt(plan)
            }
            other => other,
        })
        .collect()
}

// ---------- numbers ----------

/// A whole number too wide for the language to hold as one is a real
/// there, literal or not.
fn within_width(v: Value, lang: &Lang) -> Value {
    let places = lang.real_digits.unwrap_or(arith::DEFAULT_PLACES);
    if let (Some(bits), Value::Huge(n)) = (lang.integer_bits, &v) {
        if n.bits() >= bits as u64 {
            return arith::shape_number((**n).clone(), BigInt::from(1), Some(places));
        }
    }
    // A real written in a program is brought to the width the language
    // holds its reals in, as a real worked out while it runs is, so
    // that the two are the same number and not merely alike.
    crate::value::to_binary_width(v, lang.real_bits, places)
}

/// What a language says of a run of digits it cannot read, where it
/// gives words for that; the kernel's own naming of it otherwise.
fn unreadable_number(text: &str, lang: &Lang) -> String {
    match &lang.number_amiss {
        Some(said) => said.clone(),
        None => format!("Invalid number: {}", text),
    }
}

fn parse_number(text: &str, lang: &Lang) -> Res<Value> {
    Ok(within_width(read_number(text, lang)?, lang))
}

fn read_number(text: &str, lang: &Lang) -> Res<Value> {
    // Marks put between digits to break them up count for nothing.
    let plain: String = text.chars().filter(|c| !lang.digit_separators.contains(c)).collect();
    if plain != text {
        let (start, radix) = lang.base_prefixes.iter().find(|(p, _)| text.starts_with(p.as_str()))
            .map_or((0, 10), |(p, base)| (p.chars().count(), *base));
        let written: Vec<char> = text.chars().collect();
        for (at, c) in written.iter().enumerate() {
            if lang.digit_separators.contains(c) {
                let before = at > start && written[at - 1].is_digit(radix);
                let after_prefix = start > 0 && at == start && lang.separator_after_prefix;
                let after = written.get(at + 1).map_or(false, |d| d.is_digit(radix));
                if !(after && (before || after_prefix)) {
                    return Err(unreadable_number(text, lang));
                }
            }
        }
        return read_number(&plain, lang);
    }
    for (prefix, base) in &lang.base_prefixes {
        if let Some(digits) = text.strip_prefix(prefix.as_str()) {
            return BigInt::parse_bytes(digits.as_bytes(), *base)
                .map(Value::of_big)
                .ok_or_else(|| unreadable_number(text, lang));
        }
    }
    // A nought before more digits, where a language says so, means the
    // digits are read in base eight.
    if lang.octal_lead && text.len() > 1 && text.starts_with('0') && text.bytes().all(|b| b.is_ascii_digit()) {
        return BigInt::parse_bytes(text[1..].as_bytes(), 8)
            .map(Value::of_big)
            .ok_or_else(|| unreadable_number(text, lang));
    }
    if let Some(mark) = lang.base_mark.filter(|m| text.contains(*m)) {
        let (p, q) = in_given_base(text, mark, lang.point, lang.exponent_mark)?;
        return Ok(if q == BigInt::from(1) { Value::of_big(p) } else { arith::shape_number(p, q, Some(precision_of(text))) });
    }
    // 1e9, 2.5E-3: the part before the letter, scaled by a power of ten; always a real.
    if let Some(at) = text.find(|c| lang.exponent_letters.contains(&c)) {
        let (mantissa, power) = (&text[..at], &text[at + 1..]);
        let power: i32 = power.parse().map_err(|_| unreadable_number(text, lang))?;
        let (p, q) = match read_number(mantissa, lang)? {
            Value::Real(r) => (r.p.clone(), r.q.clone()),
            Value::Small(n) => (BigInt::from(n), BigInt::from(1)),
            Value::Huge(n) => ((*n).clone(), BigInt::from(1)),
            _ => return Err(unreadable_number(text, lang)),
        };
        let scale = BigInt::from(10).pow(power.unsigned_abs());
        let (p, q) = if power < 0 { (p, q * scale) } else { (p * scale, q) };
        return Ok(arith::shape_number(p, q, Some(precision_of(mantissa))));
    }
    if let Some(dot) = lang.point.and_then(|point| text.find(point).map(|at| (at, point))) {
        let (at, point) = dot;
        let (whole, frac) = (&text[..at], &text[at + point.len_utf8()..]);
        let scale = BigInt::from(10).pow(frac.len() as u32);
        let whole = if whole.is_empty() { BigInt::from(0) } else { decimal(whole, text)? };
        return Ok(arith::shape_number(whole * &scale + decimal(frac, text)?, scale, Some(precision_of(text))));
    }
    Ok(Value::of_big(decimal(text, text)?))
}

fn decimal(digits: &str, whole: &str) -> Res<BigInt> {
    digits.parse::<BigInt>().map_err(|_| format!("Invalid number: {}", whole))
}

/// Significant figures of a literal, fifteen at least.
fn precision_of(text: &str) -> usize {
    let digits: String = text.chars().filter(char::is_ascii_alphanumeric).collect();
    let leading_zeros = digits.chars().take_while(|c| *c == '0').count();
    digits.len().saturating_sub(leading_zeros).max(1).max(15)
}

/// `<base>@<digits>[.<fraction>][^<exponent>]`.
fn in_given_base(text: &str, mark: char, point: Option<char>, exponent: Option<char>) -> Res<(BigInt, BigInt)> {
    let cut = text.find(mark).ok_or_else(|| format!("Invalid radix-N literal: missing '{}' in '{}'", mark, text))?;
    let radix: u32 = text[..cut].parse().map_err(|_| format!("Invalid radix in literal '{}': radix must be decimal integer", text))?;
    if !(2..=36).contains(&radix) {
        return Err(format!("Invalid radix {}: must be between 2 and 36", radix));
    }
    let tail = &text[cut + mark.len_utf8()..];
    if tail.is_empty() {
        return Err(format!("Invalid radix-N literal '{}': missing digits after '{}'", text, mark));
    }
    fn cleave(s: &str, c: Option<char>) -> (&str, Option<&str>) {
        match c.and_then(|c| s.find(c).map(|p| (p, c))) {
            Some((p, c)) => (&s[..p], Some(&s[p + c.len_utf8()..])),
            None => (s, None),
        }
    }
    let (body, power) = cleave(tail, exponent);
    let (int_part, frac_part) = cleave(body, point);
    if int_part.is_empty() {
        return Err(format!("Invalid radix-N literal '{}': missing digits", text));
    }
    let complaint = |e: String| format!("Invalid radix-N literal '{}': {}", text, e);
    let mut p = digits_in(int_part, radix).map_err(complaint)?;
    let mut q = BigInt::from(1);
    match frac_part {
        Some(f) if !f.is_empty() => {
            let unit = BigInt::from(radix).pow(f.len() as u32);
            p = p * &unit + digits_in(f, radix).map_err(complaint)?;
            q = unit;
        }
        Some(_) => return Err(format!("Invalid radix-N literal '{}': missing digits after '.'", text)),
        None => {}
    }
    if let Some(e) = power {
        if e.is_empty() {
            return Err(format!("Invalid radix-N literal '{}': missing digits after exponent marker", text));
        }
        let e = digits_in(e, radix)
            .map_err(|e| format!("Invalid radix-N literal '{}': exponent {}", text, e))?
            .to_u32()
            .ok_or_else(|| format!("Invalid radix-N literal '{}': exponent too large", text))?;
        p *= BigInt::from(radix).pow(e);
    }
    Ok((p, q))
}

fn digits_in(digits: &str, base: u32) -> Res<BigInt> {
    digits.chars().try_fold(BigInt::from(0), |acc, c| {
        let d = c.to_digit(36).ok_or_else(|| format!("invalid digit '{}' for base {}", c, base))?;
        if d >= base {
            return Err(format!("digit '{}' (value {}) is not valid in base {}", c, d, base));
        }
        Ok(acc * base + d)
    })
}
