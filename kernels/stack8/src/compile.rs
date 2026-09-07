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

use crate::lang::{Lang, Brackets, Blocks};
use crate::arith;
use crate::lex::{Shape, Token};
use crate::value::Value;
use crate::code::{Operand, Builtin, Action, Routine, Cell, Instr, Plan};

/// The global names, each with a slot.
#[derive(Default)]
pub struct Registry {
    index: HashMap<String, usize>,
    pub idents: Vec<String>,
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
    instrs: Vec<Instr>,
}

pub struct Compiler<'a> {
    lang: &'a Lang,
    tokens: &'a [Token],
    pos: usize,
    registry: &'a mut Registry,
    pieces: Vec<Piece>,
    counter: usize,
    /// The class being read, and what it stands on: what `self` and
    /// `parent` mean inside a method.
    within: Option<(String, Option<String>)>,
    /// Which parameters of each program take a name's own cell rather
    /// than a copy of what it holds, found before anything is read.
    shared_args: HashMap<String, Vec<bool>>,
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
const SPARE_CELLS: [&str; 3] = ["#a", "#b", "#c"];

pub fn compile(tokens: &[Token], lang: &Lang, table: &mut Registry) -> Res<Rc<Routine>> {
    let top = Piece {
        outermost: true,
        ident: "<program>".to_string(),
        idents: Vec::new(),
        declared: Vec::new(),
        scopes: Vec::new(),
        globals: Vec::new(),
        lasts: Vec::new(),
        cycles: Vec::new(),
        escapes: Vec::new(),
        result_touched: false,
        instrs: Vec::new(),
    };
    let shared_args = shared_parameters(tokens, lang);
    let mut a = Compiler { lang, tokens, pos: 0, registry: table, pieces: vec![top], counter: 0, within: None, shared_args };
    if lang.rpn {
        a.rpn_body(&[], Span::Block)?;
        if !a.exhausted() {
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
            a.stmt()?;
            if defines {
                lifted.extend(a.piece().instrs.drain(from..));
            }
            a.skip_seps();
        }
        if !lifted.is_empty() {
            let end = a.mark();
            for at in a.piece().escapes.clone() {
                a.piece().instrs[at] = Instr::Skip(end);
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
        a.piece().instrs[at] = Instr::Skip(end);
    }
    let unit = a.pieces.pop().expect("the top unit");
    Ok(Rc::new(Routine { ident: unit.ident, formals: Vec::new(), least: 0, idents: unit.idents, returns_value: false, instrs: peephole(unit.instrs) }))
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
        self.piece().instrs[at] = Instr::Skip(here);
    }

    /// A jump to the end of the unit, patched when it closes.
    fn escape(&mut self) {
        let at = self.leap();
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

    fn read(&mut self, name: &str) {
        let slot = self.cell_to_read(name, false);
        self.put(Instr::Read(slot));
    }

    fn read_taking(&mut self, name: &str) {
        let slot = self.cell_to_read(name, true);
        self.put(Instr::Read(slot));
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

    /// Discard the top of the stack.
    fn discard(&mut self) {
        self.write(SPARE_CELLS[0]);
    }

    fn enter_cycle(&mut self, again: Option<usize>) {
        self.piece().cycles.push(Cycle { restart: again, resumes: Vec::new(), leaves: Vec::new() });
    }

    fn leave_cycle(&mut self, again: usize) {
        let lp = self.piece().cycles.pop().expect("an open loop");
        for at in lp.resumes {
            self.piece().instrs[at] = Instr::Skip(again);
        }
        for at in lp.leaves {
            self.land(at);
        }
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
        let at = self.leap();
        match self.cycle_out(levels, "break")? {
            Some(i) => self.piece().cycles[i].leaves.push(at),
            None => self.piece().escapes.push(at),
        }
        Ok(())
    }

    fn resume(&mut self, levels: usize) -> Res<()> {
        let at = self.leap();
        match self.cycle_out(levels, "continue")? {
            Some(i) => {
                let unit = self.piece();
                match unit.cycles[i].restart {
                    Some(target) => unit.instrs[at] = Instr::Skip(target),
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
            self.piece().instrs[at] = Instr::Skip(end);
        }
        let unit = self.pieces.pop().expect("the unit");
        let instrs = if returns_value && !used { relocated(unit.instrs.into_iter().skip(2).collect(), -2) } else { unit.instrs };
        Ok(Rc::new(Routine { ident: unit.ident, formals, least, idents: unit.idents, returns_value, instrs: peephole(instrs) }))
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
        self.skip_intro();
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

    fn stmt(&mut self) -> Res<()> {
        let lang = self.lang;
        if self.look().shape == Shape::Instr {
            let w = self.look().lexeme.clone();
            if !lang.let_words.is_empty() && Lang::spells(&lang.let_words, &w) {
                return self.binding();
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
                self.skip_reference();
                let name = self.want_name("after the function keyword")?;
                return self.function(name);
            }
            if Lang::spells(&lang.pass_words, &w) {
                self.take();
                return Ok(());
            }
            if Lang::spells(&lang.class_words, &w) || Lang::spells(&lang.interface_words, &w) {
                return self.class_decl();
            }
            if Lang::spells(&lang.try_words, &w) {
                return self.attempt();
            }
            if Lang::spells(&lang.throw_words, &w) {
                self.take();
                self.expr(0)?;
                self.act(Action::Hurl, 1);
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
            if Lang::spells(&lang.global_words, &w) {
                return self.global_stmt();
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

    /// A statement without a keyword: a step, a bare call, an
    /// assignment or an expression.
    fn simple_stmt(&mut self) -> Res<()> {
        let lang = self.lang;
        if self.bump_stmt()? {
            return Ok(());
        }
        if lang.bare_calls && self.look().shape == Shape::Instr {
            // A builtin without brackets after it; echo always, since a
            // bracket after it opens a group, not its arguments.
            let w = self.look().lexeme.clone();
            let bracketed = lang.calling.as_ref().map_or(false, |c| self.look_ahead(1).is_lexeme(Shape::Sign, &c.open));
            match lang.builtins.get(&w) {
                Some(Builtin::Tell) => return self.bare_call(w),
                Some(Builtin::Append) | Some(Builtin::Replace) | Some(Builtin::Define) | Some(Builtin::Pack) | Some(Builtin::Erase) | None => {}
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

    /// `global a, b;`: the names mean the globals in this unit.
    fn global_stmt(&mut self) -> Res<()> {
        self.take();
        let sep = self.lang.calling.as_ref().and_then(|c| c.between.clone());
        loop {
            let name = self.want_name("after the global keyword")?;
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
    /// is assembled in the unit around this one, where the definition runs.
    fn static_stmt(&mut self) -> Res<()> {
        self.take();
        let sep = self.lang.calling.as_ref().and_then(|c| c.between.clone());
        loop {
            let name = self.want_name("after the static keyword")?;
            let hidden = self.gensym("static");
            let inner = self.pieces.pop().expect("the unit");
            if self.pieces.is_empty() {
                self.pieces.push(inner);
                return Err("static belongs inside a function".to_string());
            }
            if self.on_assign() {
                self.take();
                self.expr(0)?;
            } else {
                self.constant(Value::Null);
            }
            self.write_global(&hidden);
            self.pieces.push(inner);
            self.piece().globals.push((name, hidden));
            match &sep {
                Some(s) if self.at_symbol(s) => {
                    self.take();
                }
                _ => return Ok(()),
            }
        }
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
        let named = match (self.look().shape, self.look_ahead(1).shape) {
            (Shape::Instr, Shape::Instr) if Lang::spells(&lang.foreach_as_words, &self.look_ahead(1).lexeme) => {
                Some(self.take().lexeme)
            }
            _ => None,
        };
        let bag = match &named {
            Some(name) => name.clone(),
            None => {
                self.expr(0)?;
                let bag = self.gensym("bag");
                self.write(&bag);
                bag
            }
        };
        if !self.on_keyword(&lang.foreach_as_words) {
            return Err(format!("Expected '{}' in foreach, got '{}'", lang.foreach_as_words[0], self.look().lexeme));
        }
        self.take();
        let mut shared = self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m));
        if shared {
            self.take();
        }
        let first = self.want_name("as the foreach variable")?;
        let paired = lang.pair_mark.as_ref().map_or(false, |m| self.at_symbol(m));
        let (key, value) = if paired {
            self.take();
            if self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) {
                self.take();
                shared = true;
            }
            (Some(first), self.want_name("as the foreach value")?)
        } else {
            (None, first)
        };
        if shared && named.is_none() {
            return Err("A foreach that hands out its items for writing needs a named array".to_string());
        }
        self.want_sign(&group.close, "after the foreach names")?;
        self.walk(&bag, key.as_deref(), &value, shared)
    }

    /// The walk itself: a place counted up to the extent, the key and
    /// the value bound from it at the head of each pass.
    fn walk(&mut self, bag: &str, key: Option<&str>, value: &str, shared: bool) -> Res<()> {
        let at = self.gensym("at");
        self.constant(Value::Small(0));
        self.write(&at);
        let extent = self.gensym("extent");
        self.read(bag);
        self.act(Action::Extent, 1);
        self.write(&extent);
        let to_test = self.leap();
        let top = self.mark();
        self.enter_cycle(None);
        if let Some(key) = key {
            self.read(bag);
            self.read(&at);
            self.act(Action::KeyAt, 2);
            self.write(key);
        }
        if shared {
            // The name is fastened to the item's own cell.
            self.read(&at);
            let held = self.cell_to_read(bag, false);
            self.put(Instr::BondItem(held));
            let name = self.cell_to_write(value);
            self.put(Instr::Fasten(name));
        } else {
            self.read(bag);
            self.read(&at);
            self.act(Action::ValueAt, 2);
            self.write(value);
        }
        self.body()?;
        let again = self.mark();
        self.read(&at);
        self.constant(Value::Small(1));
        self.act(Action::Add, 2);
        self.write(&at);
        self.land(to_test);
        self.read(&at);
        self.read(&extent);
        self.act(Action::Lt, 2);
        self.loop_back(top);
        self.leave_cycle(again);
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
        let which = lang.block_opens.iter().position(|o| self.at_lexeme(o));
        let Some(i) = which else {
            return Err(format!("Expected '{}' to open the switch, got '{}'", lang.block_opens[0], self.look().lexeme));
        };
        self.take();
        let close = lang.block_closes[i].clone();
        let mut stops = vec![close.clone()];
        stops.extend(lang.case_words.iter().cloned());
        stops.extend(lang.default_words.iter().cloned());
        self.enter_cycle(None);
        // A failed test waits for the next test; a body's end waits for
        // the next body.
        let mut failed: Option<usize> = None;
        let mut fell: Option<usize> = None;
        let mut default_at: Option<usize> = None;
        self.skip_seps();
        while !self.at_lexeme(&close) && !self.exhausted() {
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
        self.want_lexeme(&close)?;
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
            Some(Action::Add)
        } else if self.lang.decrements.iter().any(|s| *s == tok.lexeme) {
            Some(Action::Sub)
        } else {
            None
        }
    }

    /// x = x + 1 (or - 1), the name left in the slot.
    fn bump(&mut self, name: &str, op: Action) {
        self.read(name);
        self.constant(Value::Small(1));
        self.act(op, 2);
        self.write(name);
    }

    /// `++x;` or `x++;` as a statement: the value is not wanted, so
    /// both orders come to the same thing.
    fn bump_stmt(&mut self) -> Res<bool> {
        let (first, second) = (self.look().clone(), self.look_ahead(1).clone());
        if let (Some(op), Shape::Instr) = (self.bump_of(&first), second.shape) {
            self.take();
            self.take();
            self.bump(&second.lexeme, op);
            return Ok(true);
        }
        if let (Shape::Instr, Some(op)) = (first.shape, self.bump_of(&second)) {
            if self.lang.keywords.contains(&first.lexeme) || self.lang.builtins.contains_key(&first.lexeme) {
                return Ok(false);
            }
            self.take();
            self.take();
            self.bump(&first.lexeme, op);
            return Ok(true);
        }
        Ok(false)
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
                return self.function(name);
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
        self.leave_cycle(test);
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
        let var = self.want_name("as the loop variable")?;
        if !self.on_keyword(&lang.in_words) {
            return Err(format!("Expected '{}' after for loop variable, got: {}", lang.in_words[0], self.look().lexeme));
        }
        self.take();
        let range_call = self.look().shape == Shape::Instr
            && lang.builtins.get(&self.look().lexeme) == Some(&Builtin::Span)
            && lang.calling.as_ref().map_or(false, |c| self.look_ahead(1).is_lexeme(Shape::Sign, &c.open));
        if range_call {
            self.take();
            let call = lang.calling.clone().expect("call brackets");
            self.take();
            self.expr(0)?;
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
                return self.walk(&bag, None, &var, false);
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
        self.leave_cycle(again);
        Ok(())
    }

    fn return_stmt(&mut self) -> Res<()> {
        self.take();
        if self.on_sep() || self.exhausted() || self.on_any(&self.lang.block_closes) {
            self.constant(Value::Null);
        } else {
            self.expr(0)?;
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
        let (mut fields, mut shared, mut constants) = (Vec::new(), Vec::new(), Vec::new());
        let mut methods = Vec::new();
        let outer = self.within.replace((name.clone(), base.clone()));
        let which = lang.block_opens.iter().position(|o| self.at_lexeme(o));
        let Some(i) = which else {
            return Err(format!("Expected '{}' to open the class, got '{}'", lang.block_opens[0], self.look().lexeme));
        };
        self.take();
        let close = lang.block_closes[i].clone();
        self.skip_seps();
        while !self.at_lexeme(&close) && !self.exhausted() {
            let mut own = false;
            while self.look().shape == Shape::Instr {
                let w = self.look().lexeme.clone();
                if Lang::spells(&lang.shared_words, &w) {
                    own = true;
                } else if !Lang::spells(&lang.modifier_words, &w) {
                    break;
                }
                self.take();
            }
            if self.on_keyword(&lang.const_words) {
                self.take();
                let member = self.want_name("as the constant name")?;
                self.expect_assign("after the constant name")?;
                constants.push((member, self.member_value()?));
            } else if self.on_keyword(&lang.function_words) {
                self.take();
                self.skip_reference();
                let member = self.want_name("as the method name")?;
                methods.push((member.clone(), self.method(&member)?));
            } else {
                // A property, perhaps with a type word before its name.
                if self.look().shape == Shape::Instr && self.look_ahead(1).shape == Shape::Instr {
                    self.take();
                }
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
                    shared.push((bare, value));
                } else {
                    fields.push((bare, value));
                }
            }
            self.skip_seps();
        }
        self.want_lexeme(&close)?;
        self.within = outer;
        // The class it stands on first, then a value for every property,
        // every value of its own and every constant, in that order.
        let mut argc = 0;
        if let Some(base) = &base {
            self.read(base);
            argc += 1;
        }
        for named in &answers {
            self.read(named);
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
        let plan = Plan { name: name.clone(), answers: answers.len(), field_names, shared_names, constant_names, methods, extends: base.is_some() };
        self.act(Action::Forge(Rc::new(plan)), argc);
        self.write_global(&name);
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
        let lang = self.lang;
        let call = lang.calling.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.want_sign(&call.open, "after method name")?;
        let mut formals = vec![lang.this_word.clone().ok_or_else(|| "A class needs ext.stmt.class.this".to_string())?];
        let (params, spares) = self.parameters(&call)?;
        formals.extend(params);
        // The object it is for is always given, so the count is one more.
        let least = formals.len() - spares.len();
        let given = formals.clone();
        let spares: Vec<(usize, usize)> = spares.into_iter().map(|(at, from)| (at + 1, from)).collect();
        if self.look().shape == Shape::Sign && Lang::spells(&lang.return_marks, &self.look().lexeme) {
            self.take();
            self.skip_nothing_mark();
            self.want_name("as a return type")?;
        }
        // A method may be named and not written out, in a class of
        // method names only; it answers with nothing.
        if self.on_sep() {
            return self.routine(name, formals, least, true, |a| {
                a.constant(Value::Null);
                a.piece().result_touched = true;
                a.write(RESULT_CELL);
                Ok(())
            });
        }
        self.routine(name, formals, least, true, |a| {
            a.spare_values(&spares, &given)?;
            a.body()
        })
    }

    /// The parameters of a function or a method, up to the closing bracket.
    fn parameters(&mut self, call: &Brackets) -> Res<(Vec<String>, Vec<(usize, usize)>)> {
        let lang = self.lang;
        let mut formals = Vec::new();
        // Which parameter, and where its own value is written: it is read
        // again inside the program, where its names mean what they should.
        let mut spares: Vec<(usize, usize)> = Vec::new();
        while !self.at_symbol(&call.close) && !self.exhausted() {
            self.skip_reference();
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
                }
                if self.look().shape == Shape::Instr && self.look_ahead(1).shape == Shape::Instr {
                    self.take();
                }
                formals.push(self.want_name("as a parameter name")?);
                if self.look().shape == Shape::Sign && Lang::spells(&lang.type_marks, &self.look().lexeme) {
                    self.take();
                    self.want_name("as a type name")?;
                }
            }
            // A parameter may carry a value of its own for calls that
            // leave it out.
            if self.on_assign() {
                self.take();
                spares.push((formals.len() - 1, self.pos));
                // Read once here only to step over it.
                self.member_value()?;
            }
            if let Some(sep) = &call.between {
                if self.at_symbol(sep) {
                    self.take();
                }
            }
            if self.look().shape == Shape::Sign && lang.ends_stmt(&self.look().lexeme) {
                self.take();
            }
        }
        self.want_sign(&call.close, "after parameters")?;
        Ok((formals, spares))
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
    fn skip_reference(&mut self) {
        if self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) {
            self.take();
        }
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

    fn function(&mut self, name: String) -> Res<()> {
        let lang = self.lang;
        let call = lang.calling.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.want_sign(&call.open, "after function name")?;
        let (formals, spares) = self.parameters(&call)?;
        let least = formals.len() - spares.len();
        let given = formals.clone();
        if self.look().shape == Shape::Sign && Lang::spells(&lang.return_marks, &self.look().lexeme) {
            self.take();
            self.skip_nothing_mark();
            self.want_name("as a return type")?;
        }
        let declarations = self.look().shape == Shape::Sign && lang.ends_stmt(&self.look().lexeme);
        let program = self.routine(&name, formals, least, true, |a| {
            a.spare_values(&spares, &given)?;
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
            a.body()
        })?;
        self.constant(Value::Routine(program));
        self.write(&name);
        Ok(())
    }

    /// An assignment, an indexed assignment, or an expression statement.
    fn assign_or_expr(&mut self) -> Res<()> {
        let from = self.mark();
        self.expr(0)?;
        let compound = self.lang.compound.get(&self.look().lexeme).filter(|_| self.look().shape == Shape::Sign).cloned();
        if !self.on_assign() && compound.is_none() {
            self.piece().result_touched = true;
            self.write(RESULT_CELL);
            return Ok(());
        }
        let assign = self.take().lexeme;
        // The target came out as a load; turn it into a store.
        let target: Vec<Instr> = self.piece().instrs.drain(from..).collect();
        match target.as_slice() {
            // `b = &a`: b is fastened to a's cell, not given a copy.
            [Instr::Read(slot)]
                if !slot.moving
                    && compound.is_none()
                    && self.lang.reference_mark.as_ref().map_or(false, |m| self.at_symbol(m)) =>
            {
                let name = slot.ident.to_string();
                self.take();
                let source = self.want_name("as the name to share a cell with")?;
                let shared = self.cell_to_write(&source);
                self.put(Instr::Bond(shared));
                let held = self.cell_to_write(&name);
                self.put(Instr::Fasten(held));
                Ok(())
            }
            [Instr::Read(slot)] if !slot.moving => {
                let name = slot.ident.to_string();
                if let Some(op) = compound {
                    // x op= e is x = x op e.
                    self.read(&name);
                    self.expr(0)?;
                    self.act(op, 2);
                } else {
                    self.expr(0)?;
                }
                self.write(&name);
                Ok(())
            }
            _ if compound.is_some() => Err(format!("'{}' needs a plain variable on its left", assign)),
            // The read of a member turns into a write of it.
            [rest @ .., Instr::Act(Action::Grab(member), 1)] => {
                let (member, rest) = (member.clone(), rest.to_vec());
                for w in relocated(rest, 0) {
                    self.put(w);
                }
                self.expr(0)?;
                self.act(Action::Plant(member), 2);
                Ok(())
            }
            [rest @ .., Instr::Act(Action::Reach(member), 1)] => {
                let (member, rest) = (member.clone(), rest.to_vec());
                for w in relocated(rest, 0) {
                    self.put(w);
                }
                self.expr(0)?;
                self.act(Action::Sow(member), 2);
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
                self.expr(0)?;
                self.read(&name);
                self.act(Action::Grab(member.clone()), 1);
                let native = if appending { Builtin::Append } else { Builtin::Replace };
                let argc = if appending { 2 } else { 3 };
                self.act(Action::Builtin(native, Rc::from("put")), argc);
                self.act(Action::Plant(member), 2);
                Ok(())
            }
            [Instr::Read(slot), Instr::Act(Action::AtEnd, 1)] if !slot.moving => {
                // a[] = v appends.
                let name = slot.ident.to_string();
                self.expr(0)?;
                self.read_taking(&name);
                self.act(Action::Builtin(Builtin::Append, Rc::from("push")), 2);
                self.restore(&name);
                Ok(())
            }
            [Instr::Read(slot), index @ .., Instr::Act(Action::At, 2)] if !slot.moving => {
                let name = slot.ident.to_string();
                for w in relocated(index.to_vec(), -1) {
                    self.put(w);
                }
                self.expr(0)?;
                self.read_taking(&name);
                self.act(Action::Builtin(Builtin::Replace, Rc::from("put")), 3);
                self.restore(&name);
                Ok(())
            }
            _ => Err(format!("Invalid assignment target before '{}'", assign)),
        }
    }

    // ---------- expressions ----------

    fn expr(&mut self, floor: u32) -> Res<()> {
        let lang = self.lang;
        let from = self.mark();
        self.prefix()?;
        loop {
            let t = self.look();
            if !matches!(t.shape, Shape::Sign | Shape::Instr) {
                break;
            }
            let text = t.lexeme.clone();
            if Lang::spells(&lang.pipe_words, &text) {
                if lang.precedence.get(&text).copied().unwrap_or(0) < floor {
                    break;
                }
                self.take();
                self.pipe_target(from)?;
                continue;
            }
            if self.on_keyword(&lang.instanceof_words) {
                self.take();
                let named = self.want_name("as the class to test against")?;
                self.act(Action::Kindred(Rc::from(named.as_str())), 1);
                continue;
            }
            let Some(infix) = lang.dyadic.get(&text).cloned() else {
                if floor == 0 && lang.ternary.as_ref().map_or(false, |(q, _)| self.at_symbol(q)) {
                    self.ternary()?;
                    continue;
                }
                break;
            };
            if infix.level < floor {
                break;
            }
            self.take();
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
                    self.expr(right_floor)?;
                    self.act(op, 2);
                }
            }
        }
        Ok(())
    }

    /// After a pipe: a call with the piped value first, or a bare name,
    /// a call with no other argument.
    fn pipe_target(&mut self, left: usize) -> Res<()> {
        let name = self.want_name("after the pipe")?;
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

    fn prefix(&mut self) -> Res<()> {
        let lang = self.lang;
        let from = self.mark();
        let tok = self.look().clone();
        if let (Some(op), Shape::Instr) = (self.bump_of(&tok), self.look_ahead(1).shape) {
            // ++x: the new value.
            self.take();
            let name = self.take().lexeme;
            self.bump(&name, op);
            self.read(&name);
            return Ok(());
        }
        if matches!(tok.shape, Shape::Sign | Shape::Instr) {
            if let Some(infix) = lang.monadic.get(&tok.lexeme).cloned() {
                self.take();
                self.expr(infix.level)?;
                self.act(infix.action, 1);
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
            Shape::Quote => {
                self.take();
                self.constant(Value::text(&tok.lexeme));
            }
            Shape::Instr if Lang::spells(&lang.new_words, &tok.lexeme) => {
                self.take();
                let named = self.want_name("as the class to make")?;
                self.read_class(&named)?;
                let argc = match lang.calling.clone() {
                    Some(call) if self.at_symbol(&call.open) => {
                        self.take();
                        self.arguments(&call)?
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
                            } else if native == Some(Builtin::Erase) {
                                self.forget(&call)?;
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
                                self.write_global(&name);
                                self.constant(Value::Flag(true));
                            } else {
                                let argc = self.arguments_of(&tok.lexeme, &call)?;
                                self.call(&tok.lexeme, argc)?;
                            }
                        }
                        _ => {
                            self.read(&tok.lexeme);
                            if let Some(op) = self.bump_of(&self.look().clone()) {
                                // x++: the old value stays, the slot moves on.
                                self.take();
                                self.bump(&tok.lexeme, op);
                            }
                        }
                    }
                }
            }
            Shape::Sign => {
                if let Some(group) = lang.grouping.clone() {
                    if tok.lexeme == group.open {
                        self.take();
                        self.expr(0)?;
                        self.want_sign(&group.close, "to close a group")?;
                        return self.indexing(from);
                    }
                }
                if let Some(array) = lang.array_brackets.clone() {
                    if tok.lexeme == array.open {
                        self.take();
                        let count = self.elements(&array)?;
                        self.act(Action::MakeArray, count);
                        return self.indexing(from);
                    }
                }
                if let Some(map) = lang.map_brackets.clone() {
                    if tok.lexeme == map.open {
                        self.take();
                        let count = self.elements(&map)?;
                        self.act(Action::MakeMap, count);
                        return self.indexing(from);
                    }
                }
                return Err(format!("Unexpected token: {}", tok.lexeme));
            }
            _ => return Err("Expected an expression".to_string()),
        }
        self.indexing(from)
    }

    /// The class a name stands for: `self` and `parent` name the class
    /// being read and the one it stands on.
    fn read_class(&mut self, name: &str) -> Res<()> {
        let lang = self.lang;
        if Lang::spells(&lang.self_words, name) {
            let (here, _) = self.within.clone().ok_or_else(|| format!("'{}' belongs inside a class", name))?;
            self.read(&here);
            return Ok(());
        }
        if Lang::spells(&lang.parent_words, name) {
            let (here, base) = self.within.clone().ok_or_else(|| format!("'{}' belongs inside a class", name))?;
            let base = base.ok_or_else(|| format!("Class {} stands on nothing", here))?;
            self.read(&base);
            return Ok(());
        }
        self.read(name);
        Ok(())
    }

    /// `unset(a, b[k])`: each name is left as though nothing were ever
    /// written to it, and each place named is taken out of its array.
    fn forget(&mut self, call: &Brackets) -> Res<()> {
        while !self.at_symbol(&call.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", call.close));
            }
            let from = self.mark();
            self.expr(0)?;
            let named: Vec<Instr> = self.piece().instrs.drain(from..).collect();
            match named.as_slice() {
                [Instr::Read(slot)] => {
                    let held = self.cell_to_write(&slot.ident.to_string());
                    self.put(Instr::Forget(held));
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
                _ => return Err("Only a name or a place in an array can be forgotten".to_string()),
            }
            if let Some(sep) = &call.between {
                if self.at_symbol(sep) {
                    self.take();
                }
            }
        }
        self.take();
        self.constant(Value::Null);
        Ok(())
    }

    /// `expr[i]`, `expr->member` and `expr::member`, repeatable.
    fn indexing(&mut self, from: usize) -> Res<()> {
        let lang = self.lang;
        loop {
            let member = lang.member_mark.as_ref().map_or(false, |m| self.at_symbol(m));
            let scope = lang.scope_mark.as_ref().map_or(false, |m| self.at_symbol(m));
            if !member && !scope {
                break;
            }
            self.take();
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
        while self.at_symbol(&index.open) {
            // `a[]`: the place after the last, which only a store reaches.
            if self.lang.append_index && self.look_ahead(1).is_lexeme(Shape::Sign, &index.close) {
                self.take();
                self.take();
                self.act(Action::AtEnd, 1);
                continue;
            }
            self.take();
            self.expr(0)?;
            self.want_sign(&index.close, "after array index")?;
            self.act(Action::At, 2);
            stepped = true;
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
    fn elements(&mut self, pair: &Brackets) -> Res<usize> {
        let mark = self.lang.pair_mark.clone();
        let mut count = 0;
        while !self.at_symbol(&pair.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", pair.close));
            }
            self.expr(0)?;
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
            let lone = self.look().shape == Shape::Instr
                && (self.look_ahead(1).is_lexeme(Shape::Sign, &pair.close)
                    || pair.between.as_ref().map_or(false, |s| self.look_ahead(1).is_lexeme(Shape::Sign, s)));
            if shared.get(count).copied().unwrap_or(false) && lone {
                let name = self.take().lexeme;
                let cell = self.cell_to_write(&name);
                self.put(Instr::Bond(cell));
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
        let mut count = 0;
        while !self.at_symbol(&pair.close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", pair.close));
            }
            let labelled = self.look().shape == Shape::Instr
                && self.look_ahead(1).shape == Shape::Sign
                && Lang::spells(&self.lang.argument_labels, &self.look_ahead(1).lexeme);
            if labelled {
                self.pos += 2;
            }
            self.expr(0)?;
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

    /// A call by name, the arguments already on the stack: a builtin of
    /// the definition, or the program bound to the name.
    fn call(&mut self, name: &str, argc: usize) -> Res<()> {
        match self.lang.builtins.get(name).copied() {
            Some(Builtin::Append) | Some(Builtin::Replace) => {
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
        if argc != wanted {
            return Err(format!("{}() expects {} arguments, got {}", name, wanted, argc));
        }
        self.read_taking(target);
        self.act(Action::Builtin(native, Rc::from(name)), argc);
        self.restore(target);
        self.constant(Value::Null);
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
    let comparison = |op: &Action| matches!(op, Action::Eq | Action::Ne | Action::Lt | Action::Le | Action::Gt | Action::Ge);
    let arithmetic = |op: &Action| comparison(op) || matches!(op, Action::Add | Action::Sub | Action::Mul | Action::Div | Action::DivReal | Action::IntDiv | Action::Mod | Action::Power | Action::Join | Action::At);
    let mut out: Vec<Instr> = Vec::with_capacity(instrs.len());
    let mut map = vec![0usize; instrs.len() + 1];
    let mut i = 0;
    while i < instrs.len() {
        let clear = |width: usize| i + width <= instrs.len() && !targets[i + 1..i + width].iter().any(|&t| t);
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
            _ => {}
        }
    }
    out
}

/// Words moved by `delta`, their jump targets moved with them.
/// Which parameters of each program are written with the reference sign,
/// found by reading the tokens before anything is compiled: a call has to
/// know before it works out its arguments, and a program may be called
/// above where it is written.
fn shared_parameters(tokens: &[Token], lang: &Lang) -> HashMap<String, Vec<bool>> {
    let mut found = HashMap::new();
    let (Some(mark), Some(call)) = (&lang.reference_mark, &lang.calling) else { return found };
    let sign = |t: &Token, text: &str| t.is_lexeme(Shape::Sign, text);
    let mut i = 0;
    while i + 2 < tokens.len() {
        let t = &tokens[i];
        if t.shape != Shape::Instr || !Lang::spells(&lang.function_words, &t.lexeme) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if sign(&tokens[j], mark) {
            j += 1;
        }
        if tokens[j].shape != Shape::Instr || !sign(&tokens[j + 1], &call.open) {
            i += 1;
            continue;
        }
        let name = tokens[j].lexeme.clone();
        j += 2;
        let (mut marks, mut shared, mut anything, mut defaulting, mut depth) = (Vec::new(), false, false, false, 1usize);
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
                    (shared, defaulting) = (false, false);
                } else if p.shape == Shape::Sign && Lang::spells(&lang.assign_words, &p.lexeme) {
                    defaulting = true;
                } else if !defaulting && sign(p, mark) {
                    shared = true;
                }
            }
            j += 1;
        }
        if anything {
            marks.push(shared);
        }
        found.insert(name, marks);
        i = j;
    }
    found
}

fn relocated(instrs: Vec<Instr>, delta: i64) -> Vec<Instr> {
    instrs
        .into_iter()
        .map(|w| match w {
            Instr::Skip(t) => Instr::Skip((t as i64 + delta) as usize),
            Instr::Guard(t) => Instr::Guard((t as i64 + delta) as usize),
            other => other,
        })
        .collect()
}

// ---------- numbers ----------

fn parse_number(text: &str, lang: &Lang) -> Res<Value> {
    if let Some(digits) = lang.hex_prefix.as_ref().and_then(|p| text.strip_prefix(p.as_str())) {
        return BigInt::parse_bytes(digits.as_bytes(), 16).map(Value::of_big).ok_or_else(|| format!("Invalid number: {}", text));
    }
    if let Some(mark) = lang.base_mark.filter(|m| text.contains(*m)) {
        let (p, q) = in_given_base(text, mark, lang.point, lang.exponent_mark)?;
        return Ok(if q == BigInt::from(1) { Value::of_big(p) } else { arith::shape_number(p, q, Some(precision_of(text))) });
    }
    // 1e9, 2.5E-3: the part before the letter, scaled by a power of ten; always a real.
    if let Some(at) = text.find(|c| lang.exponent_letters.contains(&c)) {
        let (mantissa, power) = (&text[..at], &text[at + 1..]);
        let power: i32 = power.parse().map_err(|_| format!("Invalid number: {}", text))?;
        let (p, q) = match parse_number(mantissa, lang)? {
            Value::Real(r) => (r.p.clone(), r.q.clone()),
            Value::Small(n) => (BigInt::from(n), BigInt::from(1)),
            Value::Huge(n) => ((*n).clone(), BigInt::from(1)),
            _ => return Err(format!("Invalid number: {}", text)),
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
