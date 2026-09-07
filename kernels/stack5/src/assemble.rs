// Assembling: tokens to words in one syntax-directed pass. Expressions
// come out in post-order; a statement becomes its words with conditional
// jumps around them; a function or a program value becomes a program
// pushed as a constant. Names become slots here.
//
// With five words, every construct is a shape:
//   a jump            Lit false; Unless target
//   a loop            top: test; Unless exit; body; Lit false; Unless top
//   a call            args; Load f; Apply Call
//   a result          Store #result after each expression statement,
//                     Load #result at the end; return jumps to the end
//   a and b           a; Store #t; Load #t; Unless skip; b; Store #t;
//                     skip: Load #t; Apply Truth
//   arr[i] = v        i; v; Load arr taking; Apply put; Store arr
//   dup, swap, ...    stores and loads of scratch slots
//   [ 1 2 3 ]         Lit mark; 1 2 3; Apply gather

use std::collections::HashMap;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::language::{Def, Pair, Style};
use crate::numbers;
use crate::scan::{Kind, Tok};
use crate::values::Value;
use crate::words::{Native, Op, Program, Slot, Word};

/// The global names, each with a slot.
#[derive(Default)]
pub struct Table {
    index: HashMap<String, usize>,
    pub names: Vec<String>,
}

impl Table {
    pub fn slot(&mut self, name: &str) -> usize {
        if let Some(&slot) = self.index.get(name) {
            return slot;
        }
        self.index.insert(name.to_string(), self.names.len());
        self.names.push(name.to_string());
        self.names.len() - 1
    }
}

/// An open loop: where continue goes once known, and the jumps waiting.
struct Loop {
    again: Option<usize>,
    continues: Vec<usize>,
    breaks: Vec<usize>,
}

/// The program being assembled.
struct Unit {
    top: bool,
    name: String,
    names: Vec<String>,
    /// Which slots a bare block declared: found from inside that block
    /// only, never once it has closed.
    owned: Vec<bool>,
    blocks: Vec<Vec<usize>>,
    loops: Vec<Loop>,
    exits: Vec<usize>,
    words: Vec<Word>,
}

pub struct Assembler<'a> {
    def: &'a Def,
    toks: &'a [Tok],
    at: usize,
    table: &'a mut Table,
    units: Vec<Unit>,
    serial: usize,
}

type Outcome<T> = Result<T, String>;

const RESULT: &str = "#result";
const TEMP: &str = "#t";
const SCRATCH: [&str; 3] = ["#a", "#b", "#c"];

pub fn assemble(toks: &[Tok], def: &Def, table: &mut Table) -> Outcome<Rc<Program>> {
    let top = Unit {
        top: true,
        name: "<program>".to_string(),
        names: Vec::new(),
        owned: Vec::new(),
        blocks: Vec::new(),
        loops: Vec::new(),
        exits: Vec::new(),
        words: Vec::new(),
    };
    let mut a = Assembler { def, toks, at: 0, table, units: vec![top], serial: 0 };
    if def.style == Style::Postfix {
        a.postfix_body(&[])?;
        if !a.done() {
            return Err(format!("Unexpected '{}'", a.peek().text));
        }
    } else {
        a.skip_separators();
        while !a.done() {
            a.statement()?;
            a.skip_separators();
        }
    }
    let end = a.here();
    for at in a.unit().exits.clone() {
        a.unit().words[at] = Word::Unless(end);
    }
    let unit = a.units.pop().expect("the top unit");
    Ok(Rc::new(Program { name: unit.name, params: Vec::new(), names: unit.names, yields: false, words: unit.words }))
}

impl<'a> Assembler<'a> {
    // ---------- tokens ----------

    fn peek(&self) -> &Tok {
        &self.toks[self.at.min(self.toks.len() - 1)]
    }

    fn peek_at(&self, ahead: usize) -> &Tok {
        &self.toks[(self.at + ahead).min(self.toks.len() - 1)]
    }

    fn next(&mut self) -> Tok {
        let tok = self.peek().clone();
        if self.at + 1 < self.toks.len() {
            self.at += 1;
        }
        tok
    }

    fn done(&self) -> bool {
        self.peek().kind == Kind::End
    }

    fn at_symbol(&self, text: &str) -> bool {
        self.peek().is(Kind::Symbol, text)
    }

    /// A delimiter may be a word or a symbol.
    fn at_lexeme(&self, text: &str) -> bool {
        let t = self.peek();
        matches!(t.kind, Kind::Symbol | Kind::Word) && t.text == text
    }

    fn at_any(&self, list: &[String]) -> bool {
        list.iter().any(|x| self.at_lexeme(x))
    }

    fn at_keyword(&self, list: &[String]) -> bool {
        self.peek().kind == Kind::Word && Def::has(list, &self.peek().text)
    }

    fn at_separator(&self) -> bool {
        let t = self.peek();
        t.kind == Kind::Eol || (t.kind == Kind::Symbol && self.def.ends_statement(&t.text))
    }

    fn skip_separators(&mut self) {
        while !self.done() && self.at_separator() {
            self.next();
        }
    }

    fn expect_symbol(&mut self, text: &str, why: &str) -> Outcome<()> {
        if !self.at_symbol(text) {
            return Err(format!("Expected '{}' {}, got '{}'", text, why, self.peek().text));
        }
        self.next();
        Ok(())
    }

    fn expect_word(&mut self, why: &str) -> Outcome<String> {
        if self.peek().kind != Kind::Word {
            return Err(format!("Expected identifier {}, got '{}'", why, self.peek().text));
        }
        Ok(self.next().text)
    }

    fn expect_lexeme(&mut self, text: &str) -> Outcome<()> {
        if !self.at_lexeme(text) {
            return Err(format!("Expected '{}' to close a block, got '{}'", text, self.peek().text));
        }
        self.next();
        Ok(())
    }

    fn expect_closer(&mut self) -> Outcome<()> {
        if !self.at_any(&self.def.closers) {
            return Err(format!("Expected '{}' to close a block, got '{}'", self.def.closers[0], self.peek().text));
        }
        self.next();
        Ok(())
    }

    fn expect_intro(&mut self) -> Outcome<()> {
        if !self.at_any(&self.def.intros) {
            return Err(format!("Expected '{}', got '{}'", self.def.intros[0], self.peek().text));
        }
        self.next();
        Ok(())
    }

    fn skip_intro(&mut self) {
        if self.at_any(&self.def.intros) {
            self.next();
        }
    }

    // ---------- words ----------

    fn unit(&mut self) -> &mut Unit {
        self.units.last_mut().expect("an open unit")
    }

    fn emit(&mut self, word: Word) -> usize {
        let unit = self.unit();
        unit.words.push(word);
        unit.words.len() - 1
    }

    fn here(&mut self) -> usize {
        self.unit().words.len()
    }

    fn lit(&mut self, v: Value) {
        self.emit(Word::Lit(v));
    }

    fn apply(&mut self, op: Op, argc: usize) {
        self.emit(Word::Apply(op, argc));
    }

    /// A conditional jump to be patched.
    fn unless(&mut self) -> usize {
        self.emit(Word::Unless(0))
    }

    /// An unconditional jump to be patched: the index of its Unless.
    fn jump(&mut self) -> usize {
        self.lit(Value::Bool(false));
        self.unless()
    }

    fn jump_to(&mut self, target: usize) {
        self.lit(Value::Bool(false));
        self.emit(Word::Unless(target));
    }

    fn patch(&mut self, at: usize) {
        let here = self.here();
        self.unit().words[at] = Word::Unless(here);
    }

    /// A jump to the end of the unit, patched when it closes.
    fn exit(&mut self) {
        let at = self.jump();
        self.unit().exits.push(at);
    }

    /// The slot a name has outside every bare block: none at the top
    /// level, where such names are global.
    fn unblocked(unit: &Unit, name: &str) -> Option<usize> {
        (0..unit.names.len()).rev().find(|&s| unit.names[s] == name && !unit.owned[s])
    }

    /// Where a name is read: every slot of that name from the innermost
    /// open block outward, then the one outside the blocks, then the
    /// global. A closed block's slot is never found.
    fn read_slot(&mut self, name: &str, take: bool) -> Slot {
        let global = self.table.slot(name);
        let unit = self.units.last().expect("a unit");
        let mut locals = Vec::new();
        for block in unit.blocks.iter().rev() {
            locals.extend(block.iter().rev().filter(|&&s| unit.names[s] == name).copied());
        }
        locals.extend(Self::unblocked(unit, name));
        Slot { name: Rc::from(name), locals, global, take }
    }

    /// Where a name is written: the innermost slot of that name in the
    /// current block or function, made if there is none. At the top level
    /// a name outside every block is global; inside a block it is the
    /// block's own, forgotten on leaving.
    fn write_slot(&mut self, name: &str) -> Slot {
        let global = self.table.slot(name);
        let unit = self.units.last_mut().expect("a unit");
        if unit.top && unit.blocks.is_empty() {
            return Slot { name: Rc::from(name), locals: Vec::new(), global, take: false };
        }
        let found = match unit.blocks.last() {
            Some(block) => block.iter().rev().find(|&&s| unit.names[s] == name).copied(),
            None => Self::unblocked(unit, name),
        };
        let slot = found.unwrap_or_else(|| {
            unit.names.push(name.to_string());
            unit.owned.push(!unit.blocks.is_empty());
            let s = unit.names.len() - 1;
            if let Some(block) = unit.blocks.last_mut() {
                block.push(s);
            }
            s
        });
        Slot { name: Rc::from(name), locals: vec![slot], global, take: false }
    }

    fn load(&mut self, name: &str) {
        let slot = self.read_slot(name, false);
        self.emit(Word::Load(slot));
    }

    fn take(&mut self, name: &str) {
        let slot = self.read_slot(name, true);
        self.emit(Word::Load(slot));
    }

    fn store(&mut self, name: &str) {
        let slot = self.write_slot(name);
        self.emit(Word::Store(slot));
    }

    /// The store after a taking load: addressed like the load, so the
    /// hole it left is found wherever the value was.
    fn restore(&mut self, name: &str) {
        let slot = self.read_slot(name, false);
        self.emit(Word::Store(slot));
    }

    /// The program a call by name reaches. Where a language names the
    /// result after the function, a call of the function's own name inside
    /// it is the global program, not the result being built.
    fn load_callee(&mut self, name: &str) {
        let own = self.def.result_by_name && self.unit().name == name && !self.unit().top;
        let mut slot = self.read_slot(name, false);
        if own {
            slot.locals.clear();
        }
        self.emit(Word::Load(slot));
    }

    /// A hidden name no program can spell, new each time.
    fn fresh(&mut self, purpose: &str) -> String {
        self.serial += 1;
        format!("#{}{}", purpose, self.serial)
    }

    /// Discard the top of the stack.
    fn drop(&mut self) {
        self.store(SCRATCH[0]);
    }

    fn open_loop(&mut self, again: Option<usize>) {
        self.unit().loops.push(Loop { again, continues: Vec::new(), breaks: Vec::new() });
    }

    fn close_loop(&mut self, again: usize) {
        let lp = self.unit().loops.pop().expect("an open loop");
        for at in lp.continues {
            self.unit().words[at] = Word::Unless(again);
        }
        for at in lp.breaks {
            self.patch(at);
        }
    }

    fn break_out(&mut self) -> Outcome<()> {
        let at = self.jump();
        let unit = self.unit();
        match unit.loops.last_mut() {
            Some(lp) => lp.breaks.push(at),
            None if unit.top => unit.exits.push(at),
            None => return Err("break outside of loop".to_string()),
        }
        Ok(())
    }

    fn continue_on(&mut self) -> Outcome<()> {
        let at = self.jump();
        let unit = self.unit();
        match unit.loops.last_mut() {
            Some(lp) => match lp.again {
                Some(target) => unit.words[at] = Word::Unless(target),
                None => lp.continues.push(at),
            },
            None if unit.top => unit.exits.push(at),
            None => return Err("continue outside of loop".to_string()),
        }
        Ok(())
    }

    /// A program assembled in its own unit. A function keeps a result
    /// slot: null at first, each expression statement's value after, and
    /// its value is left on the stack at the end.
    fn program(&mut self, name: &str, params: Vec<String>, yields: bool, body: impl FnOnce(&mut Self) -> Outcome<()>) -> Outcome<Rc<Program>> {
        self.units.push(Unit {
            top: false,
            name: name.to_string(),
            names: params.clone(),
            owned: vec![false; params.len()],
            blocks: Vec::new(),
            loops: Vec::new(),
            exits: Vec::new(),
            words: Vec::new(),
        });
        if yields {
            self.lit(Value::Null);
            self.store(RESULT);
        }
        body(self)?;
        if yields {
            self.load(RESULT);
        }
        let end = self.here();
        for at in self.unit().exits.clone() {
            self.unit().words[at] = Word::Unless(end);
        }
        let unit = self.units.pop().expect("the unit");
        Ok(Rc::new(Program { name: unit.name, params, names: unit.names, yields, words: unit.words }))
    }

    // ---------- statements ----------

    fn statements_until(&mut self, stops: &[String]) -> Outcome<()> {
        self.skip_separators();
        while !self.at_any(stops) && !self.done() {
            self.statement()?;
            self.skip_separators();
        }
        Ok(())
    }

    /// The block after a statement head, in the language's style.
    fn block(&mut self) -> Outcome<()> {
        self.skip_intro();
        self.skip_separators();
        match self.def.style {
            Style::Indented => {
                if self.peek().kind != Kind::Open {
                    return Err(format!("Expected an indented block, got '{}'", self.peek().text));
                }
                self.next();
                self.skip_separators();
                while self.peek().kind != Kind::Close && !self.done() {
                    self.statement()?;
                    self.skip_separators();
                }
                if self.peek().kind != Kind::Close {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.next();
                Ok(())
            }
            Style::Braced => {
                let which = self.def.openers.iter().position(|o| self.at_lexeme(o));
                let Some(i) = which else {
                    return Err(format!("Expected '{}' to open a block, got '{}'", self.def.openers[0], self.peek().text));
                };
                self.next();
                let close = self.def.closers[i].clone();
                self.statements_until(std::slice::from_ref(&close))?;
                self.expect_lexeme(&close)
            }
            Style::Keyword | Style::Postfix => {
                let closers = self.def.closers.clone();
                self.statements_until(&closers)?;
                self.expect_closer()
            }
        }
    }

    fn statement(&mut self) -> Outcome<()> {
        let def = self.def;
        if self.peek().kind == Kind::Word {
            let w = self.peek().text.clone();
            if !def.lets.is_empty() && Def::has(&def.lets, &w) {
                return self.binding();
            }
            if Def::has(&def.ifs, &w) {
                return self.branch();
            }
            if Def::has(&def.whiles, &w) {
                return self.while_loop();
            }
            if Def::has(&def.untils, &w) {
                return self.until_loop();
            }
            if Def::has(&def.fors, &w) {
                return self.for_loop();
            }
            if Def::has(&def.returns, &w) {
                return self.return_statement();
            }
            if Def::has(&def.breaks, &w) {
                self.next();
                return self.break_out();
            }
            if Def::has(&def.continues, &w) {
                self.next();
                return self.continue_on();
            }
            if Def::has(&def.functions, &w) {
                self.next();
                let name = self.expect_word("after the function keyword")?;
                return self.function(name);
            }
            if Def::has(&def.passes, &w) {
                self.next();
                return Ok(());
            }
        }
        if def.style == Style::Braced && self.at_any(&def.openers) {
            return self.bare_block();
        }
        self.assignment_or_expression()
    }

    /// A bare block's bindings are forgotten on leaving it.
    fn bare_block(&mut self) -> Outcome<()> {
        self.unit().blocks.push(Vec::new());
        self.block()?;
        let bound = self.unit().blocks.pop().expect("the block");
        for s in bound {
            let name = self.unit().names[s].clone();
            let global = self.table.slot(&name);
            let slot = Slot { name: Rc::from(name.as_str()), locals: vec![s], global, take: false };
            self.lit(Value::Empty);
            self.emit(Word::Store(slot));
        }
        Ok(())
    }

    fn binding(&mut self) -> Outcome<()> {
        let def = self.def;
        if def.type_first {
            return self.typed_declaration();
        }
        self.next();
        if self.at_keyword(&def.mutables) {
            self.next();
        }
        let name = self.expect_word("after the binding keyword")?;
        if self.peek().kind == Kind::Symbol && Def::has(&def.annotation, &self.peek().text) {
            self.next();
            self.expect_word("as a type name")?;
        }
        if self.at_separator() || self.done() {
            self.lit(Value::Null);
        } else {
            self.expect_assign("in a binding")?;
            self.expression(0)?;
        }
        self.store(&name);
        Ok(())
    }

    /// C: the type word leads; a call bracket after the name is a function.
    fn typed_declaration(&mut self) -> Outcome<()> {
        self.next();
        let name = self.expect_word("after the type")?;
        if let Some(call) = &self.def.call {
            if self.at_symbol(&call.open) {
                return self.function(name);
            }
        }
        if self.at_separator() || self.done() {
            self.lit(Value::Null);
        } else {
            self.expect_assign("in a declaration")?;
            self.expression(0)?;
        }
        self.store(&name);
        Ok(())
    }

    fn at_assign(&self) -> bool {
        self.peek().kind == Kind::Symbol && Def::has(&self.def.assign, &self.peek().text)
    }

    fn expect_assign(&mut self, why: &str) -> Outcome<()> {
        if self.def.assign.is_empty() {
            return Err("This language has no assignment operator".to_string());
        }
        if !self.at_assign() {
            return Err(format!("Expected '{}' {}, got '{}'", self.def.assign[0], why, self.peek().text));
        }
        self.next();
        Ok(())
    }

    /// `if c block [elif c block]* [else block]`; one closer ends a
    /// keyword-style chain.
    fn branch(&mut self) -> Outcome<()> {
        let def = self.def;
        let keyword_style = def.style == Style::Keyword;
        self.next();
        self.expression(0)?;
        let skip = self.unless();
        if keyword_style {
            self.skip_intro();
            let mut stops = def.closers.clone();
            stops.extend(def.elifs.iter().cloned());
            stops.extend(def.elses.iter().cloned());
            self.statements_until(&stops)?;
        } else {
            self.block()?;
        }
        // An else may come after line ends.
        let mut ahead = 0;
        loop {
            let t = self.peek_at(ahead);
            if t.kind == Kind::Eol || (t.kind == Kind::Symbol && def.ends_statement(&t.text)) {
                ahead += 1;
            } else {
                break;
            }
        }
        let t = self.peek_at(ahead);
        let elif = t.kind == Kind::Word && Def::has(&def.elifs, &t.text);
        let els = t.kind == Kind::Word && Def::has(&def.elses, &t.text);
        if !elif && !els {
            if keyword_style {
                self.expect_closer()?;
            }
            self.patch(skip);
            return Ok(());
        }
        let over = self.jump();
        self.patch(skip);
        self.at += ahead;
        if elif {
            self.branch()?;
        } else {
            self.next();
            if self.at_keyword(&def.ifs) {
                self.branch()?;
            } else if keyword_style {
                let closers = def.closers.clone();
                self.statements_until(&closers)?;
                self.expect_closer()?;
            } else {
                self.block()?;
            }
        }
        self.patch(over);
        Ok(())
    }

    fn while_loop(&mut self) -> Outcome<()> {
        self.next();
        let top = self.here();
        self.expression(0)?;
        let out = self.unless();
        self.open_loop(Some(top));
        self.block()?;
        self.jump_to(top);
        self.patch(out);
        self.close_loop(top);
        Ok(())
    }

    /// `until c block`: the body first, then stop once c holds. The
    /// condition is written first but tested after the body, so its words
    /// are lifted out and put back after it.
    fn until_loop(&mut self) -> Outcome<()> {
        self.next();
        let from = self.here();
        self.expression(0)?;
        let test: Vec<Word> = self.unit().words.drain(from..).collect();
        let top = self.here();
        self.open_loop(None);
        self.block()?;
        let again = self.here();
        for w in shifted(test, again as i64 - from as i64) {
            self.emit(w);
        }
        self.emit(Word::Unless(top));
        self.close_loop(again);
        Ok(())
    }

    /// `for v in a..b block`: a counted loop with the bound in a hidden slot.
    fn for_loop(&mut self) -> Outcome<()> {
        let def = self.def;
        self.next();
        let var = self.expect_word("as the loop variable")?;
        if !self.at_keyword(&def.ins) {
            return Err(format!("Expected '{}' after for loop variable, got: {}", def.ins[0], self.peek().text));
        }
        self.next();
        let range_call = self.peek().kind == Kind::Word
            && def.natives.get(&self.peek().text) == Some(&Native::Range)
            && def.call.as_ref().map_or(false, |c| self.peek_at(1).is(Kind::Symbol, &c.open));
        if range_call {
            self.next();
            let call = def.call.clone().expect("call brackets");
            self.next();
            self.expression(0)?;
            self.store(&var);
            if let Some(sep) = &call.sep {
                self.expect_symbol(sep, "between the range bounds")?;
            }
            self.expression(0)?;
            self.expect_symbol(&call.close, "after the range")?;
        } else {
            let tier = def.ranges.iter().filter_map(|r| def.syntax_tiers.get(r)).min().copied().unwrap_or(0);
            self.expression(tier + 1)?;
            if !(self.peek().kind == Kind::Symbol && Def::has(&def.ranges, &self.peek().text)) {
                return Err("A for loop needs a range: start..end".to_string());
            }
            self.next();
            self.store(&var);
            self.expression(tier + 1)?;
        }
        let bound = self.fresh("end");
        self.store(&bound);
        self.counted_loop(&var, &bound, false)
    }

    /// The loop itself, the variable holding the start and the bound stored.
    fn counted_loop(&mut self, var: &str, bound: &str, postfix: bool) -> Outcome<()> {
        let top = self.here();
        self.load(var);
        self.load(bound);
        self.apply(Op::Lt, 2);
        let out = self.unless();
        self.open_loop(None);
        if postfix {
            let closers = self.def.closers.clone();
            self.postfix_body(&closers)?;
            self.expect_closer()?;
        } else {
            self.block()?;
        }
        let again = self.here();
        self.load(var);
        self.lit(Value::Int(1));
        self.apply(Op::Add, 2);
        self.store(var);
        self.jump_to(top);
        self.patch(out);
        self.close_loop(again);
        Ok(())
    }

    fn return_statement(&mut self) -> Outcome<()> {
        self.next();
        if self.at_separator() || self.done() || self.at_any(&self.def.closers) {
            self.lit(Value::Null);
        } else {
            self.expression(0)?;
        }
        self.exit();
        Ok(())
    }

    /// From the parameter list on: `( params ) [returns type] block`,
    /// with declarations before the body where the language has them.
    fn function(&mut self, name: String) -> Outcome<()> {
        let def = self.def;
        let call = def.call.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.expect_symbol(&call.open, "after function name")?;
        let mut params = Vec::new();
        while !self.at_symbol(&call.close) && !self.done() {
            if def.type_first {
                let type_word = self.expect_word("as a parameter type")?;
                if !Def::has(&def.lets, &type_word) {
                    return Err(format!("'{}' is not a type word", type_word));
                }
                if self.peek().kind == Kind::Word {
                    params.push(self.next().text);
                }
            } else {
                params.push(self.expect_word("as a parameter name")?);
                if self.peek().kind == Kind::Symbol && Def::has(&def.annotation, &self.peek().text) {
                    self.next();
                    self.expect_word("as a type name")?;
                }
            }
            if let Some(sep) = &call.sep {
                if self.at_symbol(sep) {
                    self.next();
                }
            }
            if self.peek().kind == Kind::Symbol && def.ends_statement(&self.peek().text) {
                self.next();
            }
        }
        self.expect_symbol(&call.close, "after parameters")?;
        if self.peek().kind == Kind::Symbol && Def::has(&def.returns_marks, &self.peek().text) {
            self.next();
            self.expect_word("as a return type")?;
        }
        let declarations = self.peek().kind == Kind::Symbol && def.ends_statement(&self.peek().text);
        let program = self.program(&name, params, true, |a| {
            if declarations {
                loop {
                    a.skip_separators();
                    if !def.type_first && a.at_keyword(&def.lets) {
                        a.binding()?;
                    } else {
                        break;
                    }
                }
            }
            a.block()
        })?;
        self.lit(Value::Program(program));
        self.store(&name);
        Ok(())
    }

    /// An assignment, an indexed assignment, or an expression statement.
    fn assignment_or_expression(&mut self) -> Outcome<()> {
        let from = self.here();
        self.expression(0)?;
        if !self.at_assign() {
            self.store(RESULT);
            return Ok(());
        }
        let assign = self.next().text;
        // The target came out as a load; turn it into a store.
        let target: Vec<Word> = self.unit().words.drain(from..).collect();
        match target.as_slice() {
            [Word::Load(slot)] if !slot.take => {
                let name = slot.name.to_string();
                self.expression(0)?;
                self.store(&name);
                Ok(())
            }
            [Word::Load(slot), index @ .., Word::Apply(Op::Index, 2)] if !slot.take => {
                let name = slot.name.to_string();
                for w in shifted(index.to_vec(), -1) {
                    self.emit(w);
                }
                self.expression(0)?;
                self.take(&name);
                self.apply(Op::Native(Native::Put, Rc::from("put")), 3);
                self.restore(&name);
                Ok(())
            }
            _ => Err(format!("Invalid assignment target before '{}'", assign)),
        }
    }

    // ---------- expressions ----------

    fn expression(&mut self, floor: u32) -> Outcome<()> {
        let def = self.def;
        let from = self.here();
        self.prefix()?;
        loop {
            let t = self.peek();
            if !matches!(t.kind, Kind::Symbol | Kind::Word) {
                break;
            }
            let text = t.text.clone();
            if Def::has(&def.pipes, &text) {
                if def.syntax_tiers.get(&text).copied().unwrap_or(0) < floor {
                    break;
                }
                self.next();
                self.pipe_target(from)?;
                continue;
            }
            let Some(infix) = def.binary.get(&text).cloned() else { break };
            if infix.tier < floor {
                break;
            }
            self.next();
            let right_floor = if infix.right { infix.tier } else { infix.tier + 1 };
            match infix.op {
                Op::And | Op::Or => {
                    // The right side runs only when the left leaves it open.
                    self.store(TEMP);
                    self.load(TEMP);
                    if matches!(infix.op, Op::Or) {
                        self.apply(Op::Not, 1);
                    }
                    let skip = self.unless();
                    self.expression(right_floor)?;
                    self.store(TEMP);
                    self.patch(skip);
                    self.load(TEMP);
                    self.apply(Op::Truth, 1);
                }
                op => {
                    self.expression(right_floor)?;
                    self.apply(op, 2);
                }
            }
        }
        Ok(())
    }

    /// After a pipe: a call with the piped value first, or a bare name,
    /// a call with no other argument.
    fn pipe_target(&mut self, left: usize) -> Outcome<()> {
        let name = self.expect_word("after the pipe")?;
        let native = self.def.natives.get(&name).copied();
        if matches!(native, Some(Native::Push) | Some(Native::Put)) {
            // arr.push(x): the piped value must be the array's name.
            let target = match &self.unit().words[left..] {
                [Word::Load(slot)] if !slot.take => slot.name.to_string(),
                _ => return Err(format!("First argument to {}() must be an array variable name", name)),
            };
            self.unit().words.truncate(left);
            let Some(call) = self.def.call.clone() else {
                return Err("This language has no call syntax".to_string());
            };
            self.expect_symbol(&call.open, "after the method name")?;
            let argc = self.arguments(&call)?;
            return self.mutation(&name, &target, argc + 1);
        }
        let mut argc = 1;
        if let Some(call) = self.def.call.clone() {
            if self.at_symbol(&call.open) {
                self.next();
                argc += self.arguments(&call)?;
            }
        }
        self.call(&name, argc)
    }

    fn prefix(&mut self) -> Outcome<()> {
        let def = self.def;
        let tok = self.peek().clone();
        if matches!(tok.kind, Kind::Symbol | Kind::Word) {
            if let Some(infix) = def.unary.get(&tok.text).cloned() {
                self.next();
                self.expression(infix.tier)?;
                self.apply(infix.op, 1);
                return Ok(());
            }
        }
        match tok.kind {
            Kind::Number => {
                self.next();
                let v = number_value(&tok.text, def)?;
                self.lit(v);
            }
            Kind::Text => {
                self.next();
                self.lit(Value::str(&tok.text));
            }
            Kind::Word => {
                self.next();
                if Def::has(&def.yes, &tok.text) {
                    self.lit(Value::Bool(true));
                } else if Def::has(&def.no, &tok.text) {
                    self.lit(Value::Bool(false));
                } else if Def::has(&def.none, &tok.text) {
                    self.lit(Value::Null);
                } else {
                    match def.call.clone() {
                        Some(call) if self.at_symbol(&call.open) => {
                            self.next();
                            let native = def.natives.get(&tok.text).copied();
                            if matches!(native, Some(Native::Push) | Some(Native::Put)) {
                                // push(arr, v), put(arr, i, v): the array is named.
                                let target = match self.peek().kind {
                                    Kind::Word => self.next().text,
                                    _ => return Err(format!("First argument to {}() must be an array variable name", tok.text)),
                                };
                                if let Some(sep) = &call.sep {
                                    if self.at_symbol(sep) {
                                        self.next();
                                    }
                                }
                                let argc = self.arguments(&call)?;
                                self.mutation(&tok.text, &target, argc + 1)?;
                            } else {
                                let argc = self.arguments(&call)?;
                                self.call(&tok.text, argc)?;
                            }
                        }
                        _ => self.load(&tok.text),
                    }
                }
            }
            Kind::Symbol => {
                if let Some(group) = def.group.clone() {
                    if tok.text == group.open {
                        self.next();
                        self.expression(0)?;
                        self.expect_symbol(&group.close, "to close a group")?;
                        return self.indexing();
                    }
                }
                if let Some(array) = def.array.clone() {
                    if tok.text == array.open {
                        self.next();
                        let count = self.arguments(&array)?;
                        self.apply(Op::Array, count);
                        return self.indexing();
                    }
                }
                return Err(format!("Unexpected token: {}", tok.text));
            }
            _ => return Err("Expected an expression".to_string()),
        }
        self.indexing()
    }

    /// `expr[i]`, repeatable.
    fn indexing(&mut self) -> Outcome<()> {
        let Some(index) = self.def.index.clone() else { return Ok(()) };
        while self.at_symbol(&index.open) {
            self.next();
            self.expression(0)?;
            self.expect_symbol(&index.close, "after array index")?;
            self.apply(Op::Index, 2);
        }
        Ok(())
    }

    /// Expressions up to the closing bracket, consumed; a label before an
    /// argument is dropped. Returns how many.
    fn arguments(&mut self, pair: &Pair) -> Outcome<usize> {
        let mut count = 0;
        while !self.at_symbol(&pair.close) {
            if self.done() {
                return Err(format!("Expected '{}'", pair.close));
            }
            let labelled = self.peek().kind == Kind::Word
                && self.peek_at(1).kind == Kind::Symbol
                && Def::has(&self.def.call_labels, &self.peek_at(1).text);
            if labelled {
                self.at += 2;
            }
            self.expression(0)?;
            count += 1;
            if let Some(sep) = &pair.sep {
                if self.at_symbol(sep) {
                    self.next();
                }
            }
        }
        self.next();
        Ok(count)
    }

    /// A call by name, the arguments already on the stack: a builtin of
    /// the definition, or the program bound to the name.
    fn call(&mut self, name: &str, argc: usize) -> Outcome<()> {
        match self.def.natives.get(name).copied() {
            Some(Native::Push) | Some(Native::Put) => {
                Err(format!("First argument to {}() must be an array variable name", name))
            }
            Some(native) => {
                self.apply(Op::Native(native, Rc::from(name)), argc);
                Ok(())
            }
            None => {
                self.load_callee(name);
                self.apply(Op::Call(Rc::from(name)), argc + 1);
                Ok(())
            }
        }
    }

    /// push and put write the named array in place: the array is taken
    /// out of its slot, rewritten, and put back; the value is null.
    fn mutation(&mut self, name: &str, target: &str, argc: usize) -> Outcome<()> {
        let native = self.def.natives[name];
        let wanted = if native == Native::Push { 2 } else { 3 };
        if argc != wanted {
            return Err(format!("{}() expects {} arguments, got {}", name, wanted, argc));
        }
        self.take(target);
        self.apply(Op::Native(native, Rc::from(name)), argc);
        self.restore(target);
        self.lit(Value::Null);
        Ok(())
    }

    // ---------- postfix ----------

    /// Words up to one of `stops` or the end. A quoted name is data for
    /// the word after it.
    fn postfix_body(&mut self, stops: &[String]) -> Outcome<()> {
        loop {
            self.skip_separators();
            if self.done() || self.at_any(stops) {
                return Ok(());
            }
            let tok = self.next();
            match tok.kind {
                Kind::Name => {
                    self.skip_separators();
                    let taker = self.next();
                    if !matches!(taker.kind, Kind::Word | Kind::Symbol) {
                        return Err(format!("The name '{}' must be followed by a word that takes it", tok.text));
                    }
                    self.named_word(&taker.text, &tok.text)?;
                }
                Kind::Number => {
                    let v = number_value(&tok.text, self.def)?;
                    self.lit(v);
                }
                Kind::Text => self.lit(Value::str(&tok.text)),
                Kind::Word | Kind::Symbol => self.postfix_word(&tok)?,
                _ => unreachable!("separators are skipped"),
            }
        }
    }

    /// The word after a quoted name.
    fn named_word(&mut self, word: &str, name: &str) -> Outcome<()> {
        let def = self.def;
        if Def::has(&def.assign, word) || Def::has(&def.lets, word) {
            self.store(name);
            return Ok(());
        }
        if Def::has(&def.fors, word) {
            let bound = self.fresh("end");
            self.store(&bound);
            self.store(name);
            return self.counted_loop(name, &bound, true);
        }
        match def.natives.get(word).copied() {
            Some(native @ (Native::Push | Native::Put)) => {
                self.take(name);
                self.apply(Op::Native(native, Rc::from(word)), if native == Native::Push { 2 } else { 3 });
                self.restore(name);
                Ok(())
            }
            _ => Err(format!("'{}' does not take a name, but '{}' was given", word, name)),
        }
    }

    /// The RPL stack words, as stores and loads of scratch slots.
    fn shuffle(&mut self, word: &str) -> bool {
        let def = self.def;
        let (a, b, c) = (SCRATCH[0], SCRATCH[1], SCRATCH[2]);
        let (pops, pushes): (&[&str], &[&str]) = if Def::has(&def.dup, word) {
            (&[a], &[a, a])
        } else if Def::has(&def.drop, word) {
            (&[a], &[])
        } else if Def::has(&def.swap, word) {
            (&[a, b], &[a, b])
        } else if Def::has(&def.over, word) {
            (&[a, b], &[b, a, b])
        } else if Def::has(&def.rot, word) {
            (&[a, b, c], &[b, a, c])
        } else {
            return false;
        };
        for name in pops {
            self.store(name);
        }
        for name in pushes {
            self.load(name);
        }
        true
    }

    fn postfix_word(&mut self, tok: &Tok) -> Outcome<()> {
        let def = self.def;
        let word = tok.text.as_str();
        if Def::has(&def.yes, word) {
            self.lit(Value::Bool(true));
            return Ok(());
        }
        if Def::has(&def.no, word) {
            self.lit(Value::Bool(false));
            return Ok(());
        }
        if Def::has(&def.none, word) {
            self.lit(Value::Null);
            return Ok(());
        }
        let closers = def.closers.clone();
        if Def::has(&def.ifs, word) {
            let skip = self.unless();
            let mut stops = closers.clone();
            stops.extend(def.elses.iter().cloned());
            self.postfix_body(&stops)?;
            if self.at_any(&def.elses) {
                self.next();
                let over = self.jump();
                self.patch(skip);
                self.postfix_body(&closers)?;
                self.patch(over);
            } else {
                self.patch(skip);
            }
            return self.expect_closer();
        }
        if Def::has(&def.whiles, word) {
            let top = self.here();
            self.postfix_body(&def.intros)?;
            self.expect_intro()?;
            let out = self.unless();
            self.open_loop(Some(top));
            self.postfix_body(&closers)?;
            self.expect_closer()?;
            self.jump_to(top);
            self.patch(out);
            self.close_loop(top);
            return Ok(());
        }
        if Def::has(&def.untils, word) {
            let top = self.here();
            self.open_loop(None);
            self.postfix_body(&def.intros)?;
            self.expect_intro()?;
            let again = self.here();
            self.postfix_body(&closers)?;
            self.expect_closer()?;
            self.emit(Word::Unless(top));
            self.close_loop(again);
            return Ok(());
        }
        if Def::has(&def.returns, word) {
            self.exit();
            return Ok(());
        }
        if Def::has(&def.breaks, word) {
            return self.break_out();
        }
        if Def::has(&def.continues, word) {
            return self.continue_on();
        }
        if Def::has(&def.fors, word) {
            return Err(format!("'{}' needs a quoted name before it", word));
        }
        if let Some(i) = def.program_open.iter().position(|o| o == word) {
            let close = def.program_close[i].clone();
            let name = self.fresh("program");
            let program = self.program(&format!("<{}>", &name[1..]), Vec::new(), false, |a| {
                a.postfix_body(std::slice::from_ref(&close))?;
                a.expect_lexeme(&close)
            })?;
            self.lit(Value::Program(program));
            return Ok(());
        }
        if let Some(array) = def.array.as_ref().filter(|a| a.open == word) {
            let close = array.close.clone();
            self.lit(Value::Mark);
            self.postfix_body(std::slice::from_ref(&close))?;
            self.expect_lexeme(&close)?;
            self.apply(Op::Gather, 0);
            return Ok(());
        }
        if self.shuffle(word) {
            return Ok(());
        }
        if Def::has(&def.eval, word) {
            self.apply(Op::Eval, 1);
            return Ok(());
        }
        if let Some(infix) = def.binary.get(word) {
            self.apply(infix.op.clone(), 2);
            return Ok(());
        }
        if let Some(infix) = def.unary.get(word) {
            self.apply(infix.op.clone(), 1);
            return Ok(());
        }
        if let Some(native) = def.natives.get(word).copied() {
            let (argc, yields) = match native {
                Native::Push | Native::Put => return Err(format!("'{}' needs a quoted name before it", word)),
                Native::Extern | Native::Range => return Err(format!("'{}' has no postfix form", word)),
                Native::Emit | Native::Print | Native::Write | Native::Fail => (1, false),
                Native::CharAt | Native::Get | Native::Real => (2, true),
                _ => (1, true),
            };
            self.apply(Op::Native(native, Rc::from(word)), argc);
            if !yields {
                self.drop();
            }
            return Ok(());
        }
        if tok.kind != Kind::Word {
            return Err(format!("Unexpected '{}'", word));
        }
        self.load(word);
        self.apply(Op::Run, 1);
        Ok(())
    }
}

/// Words moved by `delta`, their jump targets moved with them.
fn shifted(words: Vec<Word>, delta: i64) -> Vec<Word> {
    words
        .into_iter()
        .map(|w| match w {
            Word::Unless(t) => Word::Unless((t as i64 + delta) as usize),
            other => other,
        })
        .collect()
}

// ---------- numbers ----------

fn number_value(text: &str, def: &Def) -> Outcome<Value> {
    if let Some(digits) = def.hex_prefix.as_ref().and_then(|p| text.strip_prefix(p.as_str())) {
        return BigInt::parse_bytes(digits.as_bytes(), 16).map(Value::whole).ok_or_else(|| format!("Invalid number: {}", text));
    }
    if let Some(mark) = def.base_mark.filter(|m| text.contains(*m)) {
        let (p, q) = in_given_base(text, mark, def.point, def.exponent_mark)?;
        return Ok(if q == BigInt::from(1) { Value::whole(p) } else { numbers::form(p, q, Some(precision_of(text))) });
    }
    if let Some(dot) = def.point.and_then(|point| text.find(point).map(|at| (at, point))) {
        let (at, point) = dot;
        let (whole, frac) = (&text[..at], &text[at + point.len_utf8()..]);
        let scale = BigInt::from(10).pow(frac.len() as u32);
        let whole = if whole.is_empty() { BigInt::from(0) } else { decimal(whole, text)? };
        return Ok(numbers::form(whole * &scale + decimal(frac, text)?, scale, Some(precision_of(text))));
    }
    Ok(Value::whole(decimal(text, text)?))
}

fn decimal(digits: &str, whole: &str) -> Outcome<BigInt> {
    digits.parse::<BigInt>().map_err(|_| format!("Invalid number: {}", whole))
}

/// Significant figures of a literal, fifteen at least.
fn precision_of(text: &str) -> usize {
    let digits: String = text.chars().filter(char::is_ascii_alphanumeric).collect();
    let leading_zeros = digits.chars().take_while(|c| *c == '0').count();
    digits.len().saturating_sub(leading_zeros).max(1).max(15)
}

/// `<base>@<digits>[.<fraction>][^<exponent>]`.
fn in_given_base(text: &str, mark: char, point: Option<char>, exponent: Option<char>) -> Outcome<(BigInt, BigInt)> {
    let at = text.find(mark).ok_or_else(|| format!("Invalid base-N literal: missing '{}' in '{}'", mark, text))?;
    let base: u32 = text[..at].parse().map_err(|_| format!("Invalid base in literal '{}': base must be decimal integer", text))?;
    if !(2..=36).contains(&base) {
        return Err(format!("Invalid base {}: must be between 2 and 36", base));
    }
    let rest = &text[at + mark.len_utf8()..];
    if rest.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits after '{}'", text, mark));
    }
    fn split_on(s: &str, c: Option<char>) -> (&str, Option<&str>) {
        match c.and_then(|c| s.find(c).map(|p| (p, c))) {
            Some((p, c)) => (&s[..p], Some(&s[p + c.len_utf8()..])),
            None => (s, None),
        }
    }
    let (mantissa, exp) = split_on(rest, exponent);
    let (whole, frac) = split_on(mantissa, point);
    if whole.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits", text));
    }
    let trouble = |e: String| format!("Invalid base-N literal '{}': {}", text, e);
    let mut p = digits_in(whole, base).map_err(trouble)?;
    let mut q = BigInt::from(1);
    match frac {
        Some(f) if !f.is_empty() => {
            let scale = BigInt::from(base).pow(f.len() as u32);
            p = p * &scale + digits_in(f, base).map_err(trouble)?;
            q = scale;
        }
        Some(_) => return Err(format!("Invalid base-N literal '{}': missing digits after '.'", text)),
        None => {}
    }
    if let Some(e) = exp {
        if e.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits after exponent marker", text));
        }
        let e = digits_in(e, base)
            .map_err(|e| format!("Invalid base-N literal '{}': exponent {}", text, e))?
            .to_u32()
            .ok_or_else(|| format!("Invalid base-N literal '{}': exponent too large", text))?;
        p *= BigInt::from(base).pow(e);
    }
    Ok((p, q))
}

fn digits_in(digits: &str, base: u32) -> Outcome<BigInt> {
    digits.chars().try_fold(BigInt::from(0), |acc, c| {
        let d = c.to_digit(36).ok_or_else(|| format!("invalid digit '{}' for base {}", c, base))?;
        if d >= base {
            return Err(format!("digit '{}' (value {}) is not valid in base {}", c, d, base));
        }
        Ok(acc * base + d)
    })
}
