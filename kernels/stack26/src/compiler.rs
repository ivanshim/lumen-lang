// Compiling: tokens to words, in one pass.
//
// The parser is syntax-directed: as it recognises each construct it emits
// the words that carry it out, so no tree is built. Expressions are parsed
// by precedence climbing and come out in post-order; statements become
// jumps around bodies; a function or a program value becomes a compiled
// program pushed as a constant. Names are resolved here, to slots.

use std::collections::HashMap;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::code::{Builtin, Op, Place, Program, Word};
use crate::definition::{Language, Layout};
use crate::lexer::{Tk, Token};
use crate::number;
use crate::value::Value;

/// Global names, each with a slot.
#[derive(Default)]
pub struct Globals {
    pub slots: HashMap<String, usize>,
    pub names: Vec<String>,
}

impl Globals {
    pub fn slot(&mut self, name: &str) -> usize {
        if let Some(&slot) = self.slots.get(name) {
            return slot;
        }
        let slot = self.names.len();
        self.slots.insert(name.to_string(), slot);
        self.names.push(name.to_string());
        slot
    }
}

/// An open loop: where `continue` and `break` jump to, once known.
struct Loop {
    continue_to: Option<usize>,
    continues: Vec<usize>,
    breaks: Vec<usize>,
}

/// The program being compiled: its slots, open bare blocks and open loops.
struct Frame {
    top_level: bool,
    slot_names: Vec<String>,
    /// Whether each slot was declared by a bare block; such a slot is
    /// found only from inside its block, never after it closes.
    block_owned: Vec<bool>,
    /// Slots declared by each open bare block, innermost last.
    blocks: Vec<Vec<usize>>,
    loops: Vec<Loop>,
    words: Vec<Word>,
}

impl Frame {
    fn new(top_level: bool, params: &[String]) -> Frame {
        Frame {
            top_level,
            slot_names: params.to_vec(),
            block_owned: vec![false; params.len()],
            blocks: Vec::new(),
            loops: Vec::new(),
            words: Vec::new(),
        }
    }

    fn here(&self) -> usize {
        self.words.len()
    }
}

pub struct Compiler<'a> {
    lang: &'a Language,
    toks: &'a [Token],
    at: usize,
    globals: &'a mut Globals,
    frames: Vec<Frame>,
    programs: usize,
}

type Fallible<T> = Result<T, String>;

/// What a run of postfix words belongs to, which says where it stops.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Run {
    Block,
    Program,
    Line,
}

pub fn compile(tokens: &[Token], lang: &Language, globals: &mut Globals) -> Fallible<Rc<Program>> {
    let mut c = Compiler { lang, toks: tokens, at: 0, globals, frames: vec![Frame::new(true, &[])], programs: 0 };
    if lang.postfix {
        c.postfix_body(&[], Run::Block)?;
        if !c.done() {
            return Err(format!("Unexpected '{}'", c.peek().text));
        }
    } else {
        c.skip_separators();
        while !c.done() {
            c.statement()?;
            c.skip_separators();
        }
    }
    let frame = c.frames.pop().expect("top frame");
    let slots = frame.slot_names.len();
    Ok(Rc::new(Program { name: "<program>".to_string(), params: Vec::new(), names: frame.slot_names, slots, words: frame.words }))
}

impl<'a> Compiler<'a> {
    // ---------- tokens ----------

    fn peek(&self) -> &Token {
        &self.toks[self.at.min(self.toks.len() - 1)]
    }

    fn peek_at(&self, ahead: usize) -> &Token {
        &self.toks[(self.at + ahead).min(self.toks.len() - 1)]
    }

    fn next(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.at < self.toks.len() - 1 {
            self.at += 1;
        }
        tok
    }

    fn done(&self) -> bool {
        self.peek().kind == Tk::End
    }

    fn at_symbol(&self, text: &str) -> bool {
        self.peek().is(Tk::Symbol, text)
    }

    /// The token as either a word or a symbol: delimiters may be both.
    fn at_lexeme(&self, text: &str) -> bool {
        let t = self.peek();
        (t.kind == Tk::Symbol || t.kind == Tk::Word) && t.text == text
    }

    fn at_any(&self, list: &[String]) -> bool {
        list.iter().any(|x| self.at_lexeme(x))
    }

    fn at_word_in(&self, list: &[String]) -> bool {
        self.peek().kind == Tk::Word && Language::spells(list, &self.peek().text)
    }

    fn at_separator(&self) -> bool {
        let t = self.peek();
        t.kind == Tk::Eol || (t.kind == Tk::Symbol && self.lang.terminator(&t.text))
    }

    fn skip_separators(&mut self) {
        while !self.done() && self.at_separator() {
            self.next();
        }
    }

    fn expect_symbol(&mut self, text: &str, why: &str) -> Fallible<()> {
        if self.at_symbol(text) {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", text, why, self.peek().text))
        }
    }

    fn expect_word(&mut self, why: &str) -> Fallible<String> {
        if self.peek().kind == Tk::Word {
            Ok(self.next().text)
        } else {
            Err(format!("Expected identifier {}, got '{}'", why, self.peek().text))
        }
    }

    fn expect_lexeme(&mut self, text: &str) -> Fallible<()> {
        if self.at_lexeme(text) {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", text, self.peek().text))
        }
    }

    fn expect_closer(&mut self) -> Fallible<()> {
        if self.at_any(&self.lang.closers) {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", self.lang.closers[0], self.peek().text))
        }
    }

    // ---------- code ----------

    fn frame(&mut self) -> &mut Frame {
        self.frames.last_mut().expect("an open frame")
    }

    fn emit(&mut self, word: Word) -> usize {
        let frame = self.frame();
        frame.words.push(word);
        frame.words.len() - 1
    }

    fn here(&mut self) -> usize {
        self.frame().here()
    }

    /// Point an earlier jump at the current position.
    fn patch(&mut self, jump: usize) {
        let here = self.here();
        let word = &mut self.frame().words[jump];
        match word {
            Word::Jump(t) | Word::Unless(t) | Word::When(t) => *t = here,
            _ => unreachable!("patching a word that is not a jump"),
        }
    }

    fn name(text: &str) -> Rc<str> {
        Rc::from(text)
    }

    /// The slot a name has outside every bare block: the function's own,
    /// or none at the top level, where such names are global.
    fn own_slot(frame: &Frame, name: &str) -> Option<usize> {
        (0..frame.slot_names.len()).rev().find(|&s| frame.slot_names[s] == name && !frame.block_owned[s])
    }

    /// Where a name is read from: every slot of that name from the
    /// innermost open block outward, then the function's own, then the
    /// global. A slot of a block that has closed is never found.
    fn place_of(&mut self, name: &str) -> Place {
        let global = self.globals.slot(name);
        let frame = self.frames.last().expect("frame");
        let mut locals = Vec::new();
        for block in frame.blocks.iter().rev() {
            locals.extend(block.iter().rev().filter(|&&s| frame.slot_names[s] == name).copied());
        }
        locals.extend(Self::own_slot(frame, name));
        Place { locals, global }
    }

    /// Where a name is written to: the innermost slot of that name in the
    /// current block or function, made new if there is none. Outside a
    /// block at the top level every name is global; inside one, a binding
    /// is the block's own and is forgotten on leaving it.
    fn target_of(&mut self, name: &str) -> Place {
        let global = self.globals.slot(name);
        let frame = self.frames.last_mut().expect("frame");
        if frame.top_level && frame.blocks.is_empty() {
            return Place { locals: Vec::new(), global };
        }
        let existing = match frame.blocks.last() {
            Some(block) => block.iter().rev().find(|&&s| frame.slot_names[s] == name).copied(),
            None => Self::own_slot(frame, name),
        };
        let slot = match existing {
            Some(slot) => slot,
            None => {
                frame.slot_names.push(name.to_string());
                frame.block_owned.push(!frame.blocks.is_empty());
                let slot = frame.slot_names.len() - 1;
                if let Some(block) = frame.blocks.last_mut() {
                    block.push(slot);
                }
                slot
            }
        };
        Place { locals: vec![slot], global }
    }

    /// A hidden slot for a loop bound; at the top level a hidden global.
    fn hidden(&mut self, purpose: &str) -> Place {
        self.programs += 1;
        let name = format!("#{}{}", purpose, self.programs);
        self.target_of(&name)
    }

    fn store(&mut self, name: &str) {
        let place = self.target_of(name);
        self.emit(Word::Store(place));
    }

    fn load(&mut self, name: &str) {
        let place = self.place_of(name);
        self.emit(Word::Load(place, Self::name(name)));
    }

    fn open_loop(&mut self, continue_to: Option<usize>) {
        self.frame().loops.push(Loop { continue_to, continues: Vec::new(), breaks: Vec::new() });
    }

    fn close_loop(&mut self, continue_to: usize) {
        let lp = self.frame().loops.pop().expect("an open loop");
        for at in lp.continues {
            let word = &mut self.frame().words[at];
            *word = Word::Jump(continue_to);
        }
        for at in lp.breaks {
            self.patch(at);
        }
    }

    fn break_word(&mut self) -> Fallible<()> {
        let at = self.emit(Word::Jump(0));
        let top_level = self.frame().top_level;
        let frame = self.frame();
        match frame.loops.last_mut() {
            Some(lp) => lp.breaks.push(at),
            None if top_level => frame.words[at] = Word::Exit,
            None => return Err("break outside of loop".to_string()),
        }
        Ok(())
    }

    fn continue_word(&mut self) -> Fallible<()> {
        let at = self.emit(Word::Jump(0));
        let top_level = self.frame().top_level;
        let frame = self.frame();
        let target = match frame.loops.last_mut() {
            Some(lp) => match lp.continue_to {
                Some(target) => Some(target),
                None => {
                    lp.continues.push(at);
                    None
                }
            },
            None if top_level => {
                frame.words[at] = Word::Exit;
                None
            }
            None => return Err("continue outside of loop".to_string()),
        };
        if let Some(target) = target {
            frame.words[at] = Word::Jump(target);
        }
        Ok(())
    }

    /// Compile a body in a fresh program frame; `body` emits the words.
    fn program(&mut self, name: &str, params: Vec<String>, body: impl FnOnce(&mut Self) -> Fallible<()>) -> Fallible<Rc<Program>> {
        self.frames.push(Frame::new(false, &params));
        body(self)?;
        let frame = self.frames.pop().expect("frame");
        let slots = frame.slot_names.len();
        Ok(Rc::new(Program { name: name.to_string(), params, names: frame.slot_names, slots, words: frame.words }))
    }

    // ---------- statements ----------

    /// Statements up to one of `stops` (not consumed) or the end.
    fn body_until(&mut self, stops: &[String]) -> Fallible<()> {
        self.skip_separators();
        while !self.at_any(stops) && !self.done() {
            self.statement()?;
            self.skip_separators();
        }
        Ok(())
    }

    fn skip_intro(&mut self) {
        if self.at_any(&self.lang.intros) {
            self.next();
        }
    }

    /// A block after a statement head, in the language's layout.
    fn block(&mut self) -> Fallible<()> {
        self.skip_intro();
        self.skip_separators();
        match self.lang.layout {
            Layout::Indented => {
                if self.peek().kind != Tk::Open {
                    return Err(format!("Expected an indented block, got '{}'", self.peek().text));
                }
                self.next();
                self.skip_separators();
                while self.peek().kind != Tk::Close && !self.done() {
                    self.statement()?;
                    self.skip_separators();
                }
                if self.peek().kind != Tk::Close {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.next();
                Ok(())
            }
            Layout::Bracketed => {
                let i = self
                    .lang
                    .openers
                    .iter()
                    .position(|o| self.at_lexeme(o))
                    .ok_or_else(|| format!("Expected '{}' to open a block, got '{}'", self.lang.openers[0], self.peek().text))?;
                self.next();
                let close = self.lang.closers[i].clone();
                self.body_until(std::slice::from_ref(&close))?;
                self.expect_lexeme(&close)
            }
            Layout::Closed => {
                let closers = self.lang.closers.clone();
                self.body_until(&closers)?;
                self.expect_closer()
            }
        }
    }

    fn statement(&mut self) -> Fallible<()> {
        let lang = self.lang;
        if self.peek().kind == Tk::Word {
            let word = self.peek().text.clone();
            if !lang.let_words.is_empty() && Language::spells(&lang.let_words, &word) {
                return self.binding();
            }
            if Language::spells(&lang.if_words, &word) {
                return self.branch();
            }
            if Language::spells(&lang.while_words, &word) {
                return self.while_loop();
            }
            if Language::spells(&lang.until_words, &word) {
                return self.until_loop();
            }
            if Language::spells(&lang.for_words, &word) {
                return self.for_loop();
            }
            if Language::spells(&lang.return_words, &word) {
                return self.return_statement();
            }
            if Language::spells(&lang.break_words, &word) {
                self.next();
                return self.break_word();
            }
            if Language::spells(&lang.continue_words, &word) {
                self.next();
                return self.continue_word();
            }
            if Language::spells(&lang.function_words, &word) {
                self.next();
                let name = self.expect_word("after the function keyword")?;
                return self.function_from_params(name);
            }
            if Language::spells(&lang.pass_words, &word) {
                self.next();
                return Ok(());
            }
        }
        if lang.layout == Layout::Bracketed && self.at_any(&lang.openers) {
            return self.bare_block();
        }
        self.assignment_or_expression()
    }

    /// A bare block runs in its own scope: what it binds is forgotten after.
    fn bare_block(&mut self) -> Fallible<()> {
        self.frame().blocks.push(Vec::new());
        self.block()?;
        let slots = self.frame().blocks.pop().expect("block");
        if !slots.is_empty() {
            self.emit(Word::Forget { locals: slots, globals: Vec::new() });
        }
        Ok(())
    }

    fn binding(&mut self) -> Fallible<()> {
        let lang = self.lang;
        if lang.type_first {
            return self.typed_declaration();
        }
        self.next();
        if self.at_word_in(&lang.mutable_words) {
            self.next();
        }
        let name = self.expect_word("after the binding keyword")?;
        if self.peek().kind == Tk::Symbol && Language::spells(&lang.annotation, &self.peek().text) {
            self.next();
            self.expect_word("as a type name")?;
        }
        if self.at_separator() || self.done() {
            self.emit(Word::Lit(Value::Null));
        } else {
            self.expect_assign("in a binding")?;
            self.expression(0)?;
        }
        self.store(&name);
        Ok(())
    }

    /// C: the keyword is the type; a call bracket after the name defines a function.
    fn typed_declaration(&mut self) -> Fallible<()> {
        self.next();
        let name = self.expect_word("after the type")?;
        if let Some(call) = &self.lang.call {
            if self.at_symbol(&call.open) {
                return self.function_from_params(name);
            }
        }
        if self.at_separator() || self.done() {
            self.emit(Word::Lit(Value::Null));
        } else {
            self.expect_assign("in a declaration")?;
            self.expression(0)?;
        }
        self.store(&name);
        Ok(())
    }

    fn at_assign(&self) -> bool {
        self.peek().kind == Tk::Symbol && Language::spells(&self.lang.assign, &self.peek().text)
    }

    fn expect_assign(&mut self, why: &str) -> Fallible<()> {
        if self.lang.assign.is_empty() {
            return Err("This language has no assignment operator".to_string());
        }
        if self.at_assign() {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", self.lang.assign[0], why, self.peek().text))
        }
    }

    /// `if cond block [elif cond block]* [else block]`; in the keyword
    /// layout one closer ends the whole chain.
    fn branch(&mut self) -> Fallible<()> {
        let lang = self.lang;
        let closed = lang.layout == Layout::Closed;
        self.next();
        self.expression(0)?;
        let skip = self.emit(Word::Unless(0));
        if closed {
            self.skip_intro();
            let mut stops = lang.closers.clone();
            stops.extend(lang.elif_words.iter().cloned());
            stops.extend(lang.else_words.iter().cloned());
            self.body_until(&stops)?;
        } else {
            self.block()?;
        }
        // An else may follow after line ends.
        let mut ahead = 0;
        while self.peek_at(ahead).kind == Tk::Eol
            || (self.peek_at(ahead).kind == Tk::Symbol && lang.terminator(&self.peek_at(ahead).text))
        {
            ahead += 1;
        }
        let next = self.peek_at(ahead);
        let elif = next.kind == Tk::Word && Language::spells(&lang.elif_words, &next.text);
        let else_ = next.kind == Tk::Word && Language::spells(&lang.else_words, &next.text);
        if !elif && !else_ {
            if closed {
                self.expect_closer()?;
            }
            self.patch(skip);
            return Ok(());
        }
        let over = self.emit(Word::Jump(0));
        self.patch(skip);
        self.at += ahead;
        if elif {
            self.branch()?;
        } else {
            self.next();
            if self.at_word_in(&lang.if_words) {
                self.branch()?;
            } else if closed {
                let closers = lang.closers.clone();
                self.body_until(&closers)?;
                self.expect_closer()?;
            } else {
                self.block()?;
            }
        }
        self.patch(over);
        Ok(())
    }

    fn while_loop(&mut self) -> Fallible<()> {
        self.next();
        let top = self.here();
        self.expression(0)?;
        let exit = self.emit(Word::Unless(0));
        self.open_loop(Some(top));
        self.block()?;
        self.emit(Word::Jump(top));
        self.patch(exit);
        self.close_loop(top);
        Ok(())
    }

    /// `until cond block`: the body first, then stop once the condition holds.
    fn until_loop(&mut self) -> Fallible<()> {
        self.next();
        // The condition is written before the body but tested after it:
        // compile it, lift its words out, and put them back after the body.
        let start = self.here();
        self.expression(0)?;
        let condition: Vec<Word> = self.frame().words.drain(start..).collect();
        let top = self.here();
        self.open_loop(None);
        self.block()?;
        let test = self.here();
        for word in relocated(condition, test as i64 - start as i64) {
            self.emit(word);
        }
        self.emit(Word::Unless(top));
        self.close_loop(test);
        Ok(())
    }

    /// `for v in start..end block`: a counted loop, the bound in a hidden slot.
    fn for_loop(&mut self) -> Fallible<()> {
        let lang = self.lang;
        self.next();
        let var = self.expect_word("as the loop variable")?;
        if !self.at_word_in(&lang.in_words) {
            return Err(format!("Expected '{}' after for loop variable, got: {}", lang.in_words[0], self.peek().text));
        }
        self.next();
        // The range: `start..end`, or the range builtin with two arguments.
        let range_call = self.peek().kind == Tk::Word
            && lang.builtins.get(&self.peek().text) == Some(&Builtin::Range)
            && lang.call.as_ref().map_or(false, |c| self.peek_at(1).is(Tk::Symbol, &c.open));
        if range_call {
            self.next();
            let call = lang.call.clone().expect("call brackets");
            self.next();
            self.expression(0)?;
            self.store(&var);
            if let Some(sep) = &call.between {
                self.expect_symbol(sep, "between the range bounds")?;
            }
            self.expression(0)?;
            self.expect_symbol(&call.close, "after the range")?;
        } else {
            let range_tier = lang.ranges.iter().filter_map(|r| lang.special_tiers.get(r)).min().copied().unwrap_or(0);
            self.expression(range_tier + 1)?;
            if !(self.peek().kind == Tk::Symbol && Language::spells(&lang.ranges, &self.peek().text)) {
                return Err("A for loop needs a range: start..end".to_string());
            }
            self.next();
            self.store(&var);
            self.expression(range_tier + 1)?;
        }
        let limit = self.hidden("end");
        self.emit(Word::Store(limit.clone()));
        self.counted_loop(&var, limit, false)
    }

    /// The loop proper, once the variable holds the start and the bound is stored.
    fn counted_loop(&mut self, var: &str, limit: Place, postfix: bool) -> Fallible<()> {
        let top = self.here();
        self.load(var);
        self.emit(Word::Load(limit, Self::name("#end")));
        self.emit(Word::Op(Op::Lt));
        let exit = self.emit(Word::Unless(0));
        self.open_loop(None);
        if postfix {
            self.postfix_block()?;
        } else {
            self.block()?;
        }
        let step = self.here();
        self.load(var);
        self.emit(Word::Lit(Value::Int(1)));
        self.emit(Word::Op(Op::Add));
        self.store(var);
        self.emit(Word::Jump(top));
        self.patch(exit);
        self.close_loop(step);
        Ok(())
    }

    fn return_statement(&mut self) -> Fallible<()> {
        self.next();
        if self.at_separator() || self.done() || self.at_any(&self.lang.closers) {
            self.emit(Word::Lit(Value::Null));
        } else {
            self.expression(0)?;
        }
        self.emit(Word::Return);
        Ok(())
    }

    /// From the parameter list on: `( params ) [returns type] block`, with
    /// Pascal's declarations before the body allowed.
    fn function_from_params(&mut self, name: String) -> Fallible<()> {
        let lang = self.lang;
        let call = lang.call.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.expect_symbol(&call.open, "after function name")?;
        let mut params = Vec::new();
        while !self.at_symbol(&call.close) && !self.done() {
            if lang.type_first {
                let type_word = self.expect_word("as a parameter type")?;
                if !Language::spells(&lang.let_words, &type_word) {
                    return Err(format!("'{}' is not a type word", type_word));
                }
                if self.peek().kind == Tk::Word {
                    params.push(self.next().text);
                }
            } else {
                params.push(self.expect_word("as a parameter name")?);
                if self.peek().kind == Tk::Symbol && Language::spells(&lang.annotation, &self.peek().text) {
                    self.next();
                    self.expect_word("as a type name")?;
                }
            }
            if let Some(sep) = &call.between {
                if self.at_symbol(sep) {
                    self.next();
                }
            }
            if self.peek().kind == Tk::Symbol && lang.terminator(&self.peek().text) {
                self.next();
            }
        }
        self.expect_symbol(&call.close, "after parameters")?;
        if self.peek().kind == Tk::Symbol && Language::spells(&lang.returns_words, &self.peek().text) {
            self.next();
            self.expect_word("as a return type")?;
        }
        let declared = self.peek().kind == Tk::Symbol && lang.terminator(&self.peek().text);
        let program = self.program(&name, params, |c| {
            if declared {
                loop {
                    c.skip_separators();
                    if !lang.type_first && c.at_word_in(&lang.let_words) {
                        c.binding()?;
                    } else {
                        break;
                    }
                }
            }
            c.block()?;
            c.emit(Word::Exit);
            Ok(())
        })?;
        self.emit(Word::Lit(Value::Program(program)));
        self.store(&name);
        Ok(())
    }

    /// An assignment, an indexed assignment, or an expression statement.
    fn assignment_or_expression(&mut self) -> Fallible<()> {
        let start = self.here();
        self.expression(0)?;
        if !self.at_assign() {
            self.emit(Word::Result);
            return Ok(());
        }
        let assign = self.next().text;
        // The target was compiled as a load: replace it with a store.
        let words: Vec<Word> = self.frame().words.drain(start..).collect();
        match words.as_slice() {
            [Word::Load(_, name)] => {
                let name = name.to_string();
                self.expression(0)?;
                self.store(&name);
                Ok(())
            }
            [Word::Load(_, name), index @ .., Word::Op(Op::Index)] => {
                let name = name.to_string();
                for word in relocated(index.to_vec(), -1) {
                    self.emit(word);
                }
                self.expression(0)?;
                let place = self.place_of(&name);
                self.emit(Word::PutAt(place, Self::name(&name)));
                Ok(())
            }
            _ => Err(format!("Invalid assignment target before '{}'", assign)),
        }
    }

    // ---------- expressions ----------

    fn expression(&mut self, floor: u32) -> Fallible<()> {
        let lang = self.lang;
        let start = self.here();
        self.prefix()?;
        loop {
            let tok = self.peek();
            if tok.kind != Tk::Symbol && tok.kind != Tk::Word {
                break;
            }
            let text = tok.text.clone();
            if Language::spells(&lang.pipes, &text) {
                let tier = lang.special_tiers.get(&text).copied().unwrap_or(0);
                if tier < floor {
                    break;
                }
                self.next();
                self.pipe_target(start)?;
                continue;
            }
            let Some(op) = lang.binary.get(&text).cloned() else { break };
            if op.tier < floor {
                break;
            }
            self.next();
            let right_floor = if op.rightward { op.tier } else { op.tier + 1 };
            match op.op {
                Op::And | Op::Or => {
                    // Short-circuit: the right side runs only when the left leaves it open.
                    self.emit(Word::Truth);
                    self.emit(Word::Dup);
                    let jump = self.emit(if op.op == Op::And { Word::Unless(0) } else { Word::When(0) });
                    self.emit(Word::Drop);
                    self.expression(right_floor)?;
                    self.emit(Word::Truth);
                    self.patch(jump);
                }
                other => {
                    self.expression(right_floor)?;
                    self.emit(Word::Op(other));
                }
            }
        }
        Ok(())
    }

    /// After a pipe: a call taking the piped value as its first argument,
    /// or a bare name, which is a call with no other arguments.
    fn pipe_target(&mut self, left_start: usize) -> Fallible<()> {
        let name = self.expect_word("after the pipe")?;
        let mutating = matches!(self.lang.builtins.get(&name), Some(Builtin::Push) | Some(Builtin::Put));
        if mutating {
            // `arr.push(x)`: the piped value must be a bare name, the array to write.
            let target = match &self.frame().words[left_start..] {
                [Word::Load(_, n)] => n.clone(),
                _ => return Err(format!("First argument to {}() must be an array variable name", name)),
            };
            self.frame().words.truncate(left_start);
            let call = self.lang.call.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
            self.expect_symbol(&call.open, "after the method name")?;
            let argc = self.arguments(&call)?;
            return self.mutating_call(&name, &target, argc + 1);
        }
        let mut argc = 1;
        if let Some(call) = self.lang.call.clone() {
            if self.at_symbol(&call.open) {
                self.next();
                argc += self.arguments(&call)?;
            }
        }
        self.call(&name, argc)
    }

    fn prefix(&mut self) -> Fallible<()> {
        let lang = self.lang;
        let tok = self.peek().clone();
        if tok.kind == Tk::Symbol || tok.kind == Tk::Word {
            if let Some(op) = lang.unary.get(&tok.text).cloned() {
                self.next();
                self.expression(op.tier)?;
                self.emit(Word::Op(op.op));
                return Ok(());
            }
        }
        match tok.kind {
            Tk::Number => {
                self.next();
                let value = literal_number(&tok.text, lang)?;
                self.emit(Word::Lit(value));
            }
            Tk::Text => {
                self.next();
                self.emit(Word::Lit(Value::text(&tok.text)));
            }
            Tk::Word => {
                self.next();
                if Language::spells(&lang.yes_words, &tok.text) {
                    self.emit(Word::Lit(Value::Bool(true)));
                } else if Language::spells(&lang.no_words, &tok.text) {
                    self.emit(Word::Lit(Value::Bool(false)));
                } else if Language::spells(&lang.null_words, &tok.text) {
                    self.emit(Word::Lit(Value::Null));
                } else {
                    match lang.call.clone() {
                        Some(call) if self.at_symbol(&call.open) => {
                            self.next();
                            let mutating = matches!(lang.builtins.get(&tok.text), Some(Builtin::Push) | Some(Builtin::Put));
                            if mutating {
                                // push(arr, v), put(arr, i, v): the array is named, not evaluated.
                                let target = match self.peek().kind {
                                    Tk::Word => self.next().text,
                                    _ => return Err(format!("First argument to {}() must be an array variable name", tok.text)),
                                };
                                if let Some(sep) = &call.between {
                                    if self.at_symbol(sep) {
                                        self.next();
                                    }
                                }
                                let argc = self.arguments(&call)?;
                                self.mutating_call(&tok.text, &target, argc + 1)?;
                            } else {
                                let argc = self.arguments(&call)?;
                                self.call(&tok.text, argc)?;
                            }
                        }
                        _ => self.load(&tok.text),
                    }
                }
            }
            Tk::Symbol => {
                if let Some(group) = lang.group.clone() {
                    if tok.text == group.open {
                        self.next();
                        self.expression(0)?;
                        self.expect_symbol(&group.close, "to close a group")?;
                        return self.postfix_index();
                    }
                }
                if let Some(array) = lang.array.clone() {
                    if tok.text == array.open {
                        self.next();
                        let count = self.arguments(&array)?;
                        self.emit(Word::Collect(count));
                        return self.postfix_index();
                    }
                }
                return Err(format!("Unexpected token: {}", tok.text));
            }
            _ => return Err("Expected an expression".to_string()),
        }
        self.postfix_index()
    }

    /// `expr[index]`, repeatable.
    fn postfix_index(&mut self) -> Fallible<()> {
        let Some(index) = self.lang.index.clone() else { return Ok(()) };
        while self.at_symbol(&index.open) {
            self.next();
            self.expression(0)?;
            self.expect_symbol(&index.close, "after array index")?;
            self.emit(Word::Op(Op::Index));
        }
        Ok(())
    }

    /// Expressions up to the closing bracket, which is consumed; an
    /// argument label (Swift's `n:`) is dropped. Returns how many.
    fn arguments(&mut self, brackets: &crate::definition::Brackets) -> Fallible<usize> {
        let mut count = 0;
        while !self.at_symbol(&brackets.close) {
            if self.done() {
                return Err(format!("Expected '{}'", brackets.close));
            }
            let labelled = self.peek().kind == Tk::Word
                && self.peek_at(1).kind == Tk::Symbol
                && Language::spells(&self.lang.call_labels, &self.peek_at(1).text);
            if labelled {
                self.at += 2;
            }
            self.expression(0)?;
            count += 1;
            if let Some(sep) = &brackets.between {
                if self.at_symbol(sep) {
                    self.next();
                }
            }
        }
        self.next();
        Ok(count)
    }

    /// A call by name with its arguments already on the stack: a builtin
    /// of the definition, or the program bound to the name.
    fn call(&mut self, name: &str, argc: usize) -> Fallible<()> {
        match self.lang.builtins.get(name).copied() {
            Some(Builtin::Push) | Some(Builtin::Put) => {
                Err(format!("First argument to {}() must be an array variable name", name))
            }
            Some(builtin) => {
                self.emit(Word::Apply { builtin, name: Self::name(name), argc });
                Ok(())
            }
            None => {
                let place = self.place_of(name);
                self.emit(Word::Call { place, name: Self::name(name), argc });
                Ok(())
            }
        }
    }

    /// `push(arr, v)` and `put(arr, i, v)` write the named array in place;
    /// the values after the name are on the stack.
    fn mutating_call(&mut self, name: &str, target: &str, argc: usize) -> Fallible<()> {
        let builtin = self.lang.builtins[name];
        let wanted = if builtin == Builtin::Push { 2 } else { 3 };
        if argc != wanted {
            return Err(format!("{}() expects {} arguments, got {}", name, wanted, argc));
        }
        let place = self.place_of(target);
        let word = if builtin == Builtin::Push {
            Word::Append(place, Self::name(target))
        } else {
            Word::PutAt(place, Self::name(target))
        };
        self.emit(word);
        Ok(())
    }

    // ---------- postfix ----------

    /// Words up to one of `stops` or the end. A quoted name is data for the
    /// word right after it, which must take one. What else stops the words
    /// depends on what they are part of: a block ends at its Close token
    /// and refuses a deeper indentation; a program value, bracketed, lets
    /// indentation pass; a line ends at its end.
    fn postfix_body(&mut self, stops: &[String], run: Run) -> Fallible<()> {
        loop {
            match run {
                Run::Line => {
                    while self.peek().kind == Tk::Symbol && self.lang.terminator(&self.peek().text) {
                        self.next();
                    }
                }
                _ => self.skip_separators(),
            }
            if self.done() || self.at_any(stops) {
                return Ok(());
            }
            let kind = self.peek().kind;
            if kind == Tk::Open || kind == Tk::Close || kind == Tk::Eol {
                match run {
                    Run::Program => {
                        self.next();
                        continue;
                    }
                    Run::Block if kind == Tk::Open => return Err("Unexpected indentation".to_string()),
                    _ => return Ok(()),
                }
            }
            let tok = self.next();
            match tok.kind {
                Tk::Name => {
                    self.skip_separators();
                    let taker = self.next();
                    if taker.kind != Tk::Word && taker.kind != Tk::Symbol {
                        return Err(format!("The name '{}' must be followed by a word that takes it", tok.text));
                    }
                    self.named_word(&taker.text, &tok.text)?;
                }
                Tk::Number => {
                    let value = literal_number(&tok.text, self.lang)?;
                    self.emit(Word::Lit(value));
                }
                Tk::Text => {
                    self.emit(Word::Lit(Value::text(&tok.text)));
                }
                Tk::Word | Tk::Symbol => self.postfix_word(&tok)?,
                _ => unreachable!("separators, line ends and block marks are handled above"),
            }
        }
    }

    /// The body a control word governs, in the language's block style:
    /// indented lines, a bracketed run, or words up to a closer.
    fn postfix_block(&mut self) -> Fallible<()> {
        match self.lang.layout {
            Layout::Indented => {
                self.skip_separators();
                if self.peek().kind != Tk::Open {
                    return Err(format!("Expected an indented block, got '{}'", self.peek().text));
                }
                self.next();
                self.postfix_body(&[], Run::Block)?;
                if self.peek().kind != Tk::Close {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.next();
                Ok(())
            }
            Layout::Bracketed => {
                let which = self.lang.openers.iter().position(|o| self.at_lexeme(o));
                let Some(i) = which else {
                    return Err(format!("Expected '{}' to open a block, got '{}'", self.lang.openers[0], self.peek().text));
                };
                self.next();
                let close = self.lang.closers[i].clone();
                self.postfix_body(std::slice::from_ref(&close), Run::Block)?;
                self.expect_lexeme(&close)
            }
            Layout::Closed => {
                let closers = self.lang.closers.clone();
                self.postfix_body(&closers, Run::Block)?;
                self.expect_closer()
            }
        }
    }

    /// A loop's condition, run again each pass: the words up to where the
    /// body begins, which is the line end, the block opener, or the intro
    /// word by style.
    fn postfix_condition(&mut self) -> Fallible<()> {
        match self.lang.layout {
            Layout::Indented => self.postfix_body(&[], Run::Line),
            Layout::Bracketed => {
                let openers = self.lang.openers.clone();
                self.postfix_body(&openers, Run::Block)
            }
            Layout::Closed => {
                let intros = self.lang.intros.clone();
                self.postfix_body(&intros, Run::Block)?;
                self.expect_intro()
            }
        }
    }

    /// Whether an else word comes next, past line ends; if so, step onto it.
    fn take_else(&mut self) -> bool {
        let mut ahead = 0;
        while self.peek_at(ahead).kind == Tk::Eol
            || (self.peek_at(ahead).kind == Tk::Symbol && self.lang.terminator(&self.peek_at(ahead).text))
        {
            ahead += 1;
        }
        let t = self.peek_at(ahead);
        if t.kind == Tk::Word && Language::spells(&self.lang.else_words, &t.text) {
            self.at += ahead + 1;
            return true;
        }
        false
    }

    fn expect_intro(&mut self) -> Fallible<()> {
        if self.at_any(&self.lang.intros) {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}', got '{}'", self.lang.intros[0], self.peek().text))
        }
    }

    /// The word after a quoted name.
    fn named_word(&mut self, word: &str, name: &str) -> Fallible<()> {
        let lang = self.lang;
        if Language::spells(&lang.assign, word) || Language::spells(&lang.let_words, word) {
            self.store(name);
            return Ok(());
        }
        if Language::spells(&lang.for_words, word) {
            let limit = self.hidden("end");
            self.emit(Word::Store(limit.clone()));
            self.store(name);
            return self.counted_loop(name, limit, true);
        }
        match lang.builtins.get(word) {
            Some(Builtin::Push) => {
                let place = self.place_of(name);
                self.emit(Word::Append(place, Self::name(name)));
                self.emit(Word::Drop);
                Ok(())
            }
            Some(Builtin::Put) => {
                let place = self.place_of(name);
                self.emit(Word::PutAt(place, Self::name(name)));
                self.emit(Word::Drop);
                Ok(())
            }
            _ => Err(format!("'{}' does not take a name, but '{}' was given", word, name)),
        }
    }

    fn postfix_word(&mut self, tok: &Token) -> Fallible<()> {
        let lang = self.lang;
        let word = tok.text.as_str();
        if Language::spells(&lang.yes_words, word) {
            self.emit(Word::Lit(Value::Bool(true)));
            return Ok(());
        }
        if Language::spells(&lang.no_words, word) {
            self.emit(Word::Lit(Value::Bool(false)));
            return Ok(());
        }
        if Language::spells(&lang.null_words, word) {
            self.emit(Word::Lit(Value::Null));
            return Ok(());
        }
        if Language::spells(&lang.if_words, word) {
            let skip = self.emit(Word::Unless(0));
            if lang.layout == Layout::Closed {
                // One closer ends the whole if/else.
                let closers = lang.closers.clone();
                let mut stops = closers.clone();
                stops.extend(lang.else_words.iter().cloned());
                self.postfix_body(&stops, Run::Block)?;
                if self.at_any(&lang.else_words) {
                    self.next();
                    let over = self.emit(Word::Jump(0));
                    self.patch(skip);
                    self.postfix_body(&closers, Run::Block)?;
                    self.patch(over);
                } else {
                    self.patch(skip);
                }
                return self.expect_closer();
            }
            self.postfix_block()?;
            if self.take_else() {
                let over = self.emit(Word::Jump(0));
                self.patch(skip);
                self.postfix_block()?;
                self.patch(over);
            } else {
                self.patch(skip);
            }
            return Ok(());
        }
        if Language::spells(&lang.while_words, word) {
            let top = self.here();
            self.postfix_condition()?;
            let exit = self.emit(Word::Unless(0));
            self.open_loop(Some(top));
            self.postfix_block()?;
            self.emit(Word::Jump(top));
            self.patch(exit);
            self.close_loop(top);
            return Ok(());
        }
        if Language::spells(&lang.until_words, word) {
            // The condition is written first and tested after the body:
            // lifted out and put back after it, as in the infix form.
            let start = self.here();
            self.postfix_condition()?;
            let condition: Vec<Word> = self.frame().words.drain(start..).collect();
            let top = self.here();
            self.open_loop(None);
            self.postfix_block()?;
            let test = self.here();
            for word in relocated(condition, test as i64 - start as i64) {
                self.emit(word);
            }
            self.emit(Word::Unless(top));
            self.close_loop(test);
            return Ok(());
        }
        if Language::spells(&lang.return_words, word) {
            self.emit(Word::Exit);
            return Ok(());
        }
        if Language::spells(&lang.break_words, word) {
            return self.break_word();
        }
        if Language::spells(&lang.continue_words, word) {
            return self.continue_word();
        }
        if Language::spells(&lang.for_words, word) {
            return Err(format!("'{}' needs a quoted name before it", word));
        }
        if let Some(i) = lang.program_open.iter().position(|o| o == word) {
            let close = lang.program_close[i].clone();
            self.programs += 1;
            let name = format!("<program{}>", self.programs);
            let program = self.program(&name, Vec::new(), |c| {
                c.postfix_body(std::slice::from_ref(&close), Run::Program)?;
                c.expect_lexeme(&close)?;
                c.emit(Word::Exit);
                Ok(())
            })?;
            self.emit(Word::Lit(Value::Program(program)));
            return Ok(());
        }
        if let Some(array) = lang.array.as_ref().filter(|a| a.open == word) {
            let close = array.close.clone();
            self.emit(Word::Mark);
            self.postfix_body(std::slice::from_ref(&close), Run::Block)?;
            self.expect_lexeme(&close)?;
            self.emit(Word::Gather);
            return Ok(());
        }
        let stack_words = [
            (&lang.dup, Word::Dup), (&lang.drop, Word::Drop), (&lang.swap, Word::Swap),
            (&lang.over, Word::Over), (&lang.rot, Word::Rot), (&lang.eval, Word::Eval),
        ];
        for (list, w) in stack_words {
            if Language::spells(list, word) {
                self.emit(w);
                return Ok(());
            }
        }
        if let Some(op) = lang.binary.get(word) {
            self.emit(Word::Op(op.op));
            return Ok(());
        }
        if let Some(op) = lang.unary.get(word) {
            self.emit(Word::Op(op.op));
            return Ok(());
        }
        if let Some(builtin) = lang.builtins.get(word).copied() {
            let (argc, yields) = match builtin {
                Builtin::Push | Builtin::Put => return Err(format!("'{}' needs a quoted name before it", word)),
                Builtin::Extern | Builtin::Range => return Err(format!("'{}' has no postfix form", word)),
                Builtin::Emit | Builtin::Print | Builtin::Write | Builtin::Error => (1, false),
                Builtin::CharAt | Builtin::Get | Builtin::Real => (2, true),
                _ => (1, true),
            };
            self.emit(Word::Apply { builtin, name: Self::name(word), argc });
            if !yields {
                self.emit(Word::Drop);
            }
            return Ok(());
        }
        if tok.kind != Tk::Word {
            return Err(format!("Unexpected '{}'", word));
        }
        let place = self.place_of(word);
        self.emit(Word::Run(place, Self::name(word)));
        Ok(())
    }
}

/// Words moved by `delta` positions, their jump targets moved with them.
fn relocated(words: Vec<Word>, delta: i64) -> Vec<Word> {
    words
        .into_iter()
        .map(|w| match w {
            Word::Jump(t) => Word::Jump((t as i64 + delta) as usize),
            Word::Unless(t) => Word::Unless((t as i64 + delta) as usize),
            Word::When(t) => Word::When((t as i64 + delta) as usize),
            other => other,
        })
        .collect()
}

// ---------- number literals ----------

fn literal_number(text: &str, lang: &Language) -> Result<Value, String> {
    if let Some(prefix) = &lang.hex_prefix {
        if let Some(digits) = text.strip_prefix(prefix.as_str()) {
            return BigInt::parse_bytes(digits.as_bytes(), 16)
                .map(Value::integer)
                .ok_or_else(|| format!("Invalid number: {}", text));
        }
    }
    if let Some(mark) = lang.base_mark.filter(|m| text.contains(*m)) {
        let (top, bottom) = based(text, mark, lang.point, lang.exponent_mark)?;
        return Ok(if bottom == BigInt::from(1) { Value::integer(top) } else { number::make(top, bottom, Some(figures(text))) });
    }
    if let Some(point) = lang.point {
        if let Some(dot) = text.find(point) {
            let whole = &text[..dot];
            let frac = &text[dot + point.len_utf8()..];
            let scale = BigInt::from(10).pow(frac.len() as u32);
            let whole: BigInt = if whole.is_empty() { BigInt::from(0) } else { digits_of(whole, text)? };
            let top = whole * &scale + digits_of(frac, text)?;
            return Ok(number::make(top, scale, Some(figures(text))));
        }
    }
    Ok(Value::integer(digits_of(text, text)?))
}

fn digits_of(digits: &str, whole: &str) -> Result<BigInt, String> {
    digits.parse::<BigInt>().map_err(|_| format!("Invalid number: {}", whole))
}

/// Significant figures of a literal, at least 15.
fn figures(text: &str) -> usize {
    let digits: String = text.chars().filter(char::is_ascii_alphanumeric).collect();
    let zeros = digits.chars().take_while(|c| *c == '0').count();
    digits.len().saturating_sub(zeros).max(1).max(15)
}

/// `<base>@<digits>[.<fraction>][^<exponent>]`.
fn based(text: &str, mark: char, point: Option<char>, exponent: Option<char>) -> Result<(BigInt, BigInt), String> {
    let at = text.find(mark).ok_or_else(|| format!("Invalid base-N literal: missing '{}' in '{}'", mark, text))?;
    let base: u32 = text[..at].parse().map_err(|_| format!("Invalid base in literal '{}': base must be decimal integer", text))?;
    if !(2..=36).contains(&base) {
        return Err(format!("Invalid base {}: must be between 2 and 36", base));
    }
    let rest = &text[at + mark.len_utf8()..];
    if rest.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits after '{}'", text, mark));
    }
    let (mantissa, exp) = match exponent.and_then(|e| rest.find(e).map(|p| (p, e))) {
        Some((p, e)) => (&rest[..p], Some(&rest[p + e.len_utf8()..])),
        None => (rest, None),
    };
    let (whole, frac) = match point.and_then(|d| mantissa.find(d).map(|p| (p, d))) {
        Some((p, d)) => (&mantissa[..p], Some(&mantissa[p + d.len_utf8()..])),
        None => (mantissa, None),
    };
    if whole.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits", text));
    }
    let bad = |e: String| format!("Invalid base-N literal '{}': {}", text, e);
    let mut top = in_base(whole, base).map_err(bad)?;
    let mut bottom = BigInt::from(1);
    match frac {
        Some(f) if !f.is_empty() => {
            let scale = BigInt::from(base).pow(f.len() as u32);
            top = top * &scale + in_base(f, base).map_err(bad)?;
            bottom = scale;
        }
        Some(_) => return Err(format!("Invalid base-N literal '{}': missing digits after '.'", text)),
        None => {}
    }
    if let Some(e) = exp {
        if e.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits after exponent marker", text));
        }
        let e = in_base(e, base)
            .map_err(|e| format!("Invalid base-N literal '{}': exponent {}", text, e))?
            .to_u32()
            .ok_or_else(|| format!("Invalid base-N literal '{}': exponent too large", text))?;
        top *= BigInt::from(base).pow(e);
    }
    Ok((top, bottom))
}

fn in_base(digits: &str, base: u32) -> Result<BigInt, String> {
    let mut out = BigInt::from(0);
    for c in digits.chars() {
        let d = c.to_digit(36).ok_or_else(|| format!("invalid digit '{}' for base {}", c, base))?;
        if d >= base {
            return Err(format!("digit '{}' (value {}) is not valid in base {}", c, d, base));
        }
        out = out * base + d;
    }
    Ok(out)
}
