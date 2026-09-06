// Stage 3, reduce: tokens to the tree of primitives.
//
// Statements are told apart by the words the definition gives each form;
// expressions are parsed by precedence climbing over its operator tiers.
// Names are resolved here to slots. A postfix language is read with a
// symbolic stack: each word pushes or pops tree nodes rather than values,
// so `5 3 +` becomes the same node as `5 + 3`, and a program's parameters
// are the values it pops from an empty stack.

use std::collections::HashMap;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::ingest::{Kind, Token};
use crate::numeric;
use crate::spec::{Spec, Style};
use crate::tree::{Callee, Exit, Form, Native, Node, Op, Program, Slot};
use crate::value::Value;

pub type Outcome<T> = Result<T, String>;

/// Global names, each with a slot.
#[derive(Default)]
pub struct Globals {
    pub names: Vec<String>,
    index: HashMap<String, usize>,
}

impl Globals {
    pub fn slot(&mut self, name: &str) -> usize {
        if let Some(&i) = self.index.get(name) {
            return i;
        }
        self.names.push(name.to_string());
        self.index.insert(name.to_string(), self.names.len() - 1);
        self.names.len() - 1
    }
}

/// The frame being built: a program's slots, its open bare blocks, and,
/// for a postfix program, the parameters it has popped so far.
struct Frame {
    top: bool,
    slot_names: Vec<String>,
    blocks: Vec<Vec<usize>>,
    params: Vec<String>,
    param_slots: Vec<usize>,
    postfix_program: bool,
}

/// What a named postfix program takes and leaves, for calls by name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Arity {
    pub takes: usize,
    pub leaves: bool,
}

pub struct Reducer<'a> {
    spec: &'a Spec,
    toks: &'a [Token],
    at: usize,
    globals: &'a mut Globals,
    frames: Vec<Frame>,
    hidden: usize,
    /// Postfix: the arities assumed for programs called by name.
    pub arities: HashMap<String, Arity>,
    /// Postfix: the arities found for programs bound by name.
    pub found: HashMap<String, Arity>,
    /// Postfix: whether stack discipline is enforced. While the arities
    /// of the named programs are still being settled it is not, since a
    /// wrong assumption leaves values behind; the final reading enforces it.
    strict: bool,
}

pub fn reduce(tokens: &[Token], spec: &Spec, globals: &mut Globals, arities: HashMap<String, Arity>, strict: bool) -> Outcome<(Rc<Program>, HashMap<String, Arity>)> {
    let mut r = Reducer {
        spec,
        toks: tokens,
        at: 0,
        globals,
        frames: vec![Frame { top: true, slot_names: Vec::new(), blocks: Vec::new(), params: Vec::new(), param_slots: Vec::new(), postfix_program: false }],
        hidden: 0,
        arities,
        found: HashMap::new(),
        strict,
    };
    let body = if spec.style == Style::Postfix {
        let (mut stmts, rest) = r.postfix_body(&[])?;
        if !r.done() {
            return Err(format!("Unexpected '{}'", r.peek().text));
        }
        // Values left on the stack at the end: run them for their effects.
        stmts.extend(rest.into_iter().filter(|n| !pure(n)));
        Node::seq(1, stmts)
    } else {
        let mut stmts = Vec::new();
        r.skip_separators();
        while !r.done() {
            stmts.push(r.statement()?);
            r.skip_separators();
        }
        Node::seq(1, stmts)
    };
    let frame = r.frames.pop().expect("frame");
    let program = Program { name: "<program>".to_string(), params: Vec::new(), param_slots: Vec::new(), slot_names: frame.slot_names, body };
    Ok((Rc::new(program), r.found))
}

fn pure(node: &Node) -> bool {
    match &node.form {
        Form::Literal(_) | Form::Load(_) => true,
        Form::Operate { args, .. } => args.iter().all(pure),
        _ => false,
    }
}

impl<'a> Reducer<'a> {
    // ---------- tokens ----------

    fn peek(&self) -> &Token {
        &self.toks[self.at.min(self.toks.len() - 1)]
    }

    fn ahead(&self, n: usize) -> &Token {
        &self.toks[(self.at + n).min(self.toks.len() - 1)]
    }

    fn take(&mut self) -> Token {
        let t = self.peek().clone();
        if self.at + 1 < self.toks.len() {
            self.at += 1;
        }
        t
    }

    fn done(&self) -> bool {
        self.peek().kind == Kind::End
    }

    fn line(&self) -> u32 {
        self.peek().line
    }

    fn at_sym(&self, s: &str) -> bool {
        self.peek().kind == Kind::Symbol && self.peek().text == s
    }

    fn at_lex(&self, s: &str) -> bool {
        matches!(self.peek().kind, Kind::Symbol | Kind::Word) && self.peek().text == s
    }

    fn at_label(&self, label: &str) -> bool {
        self.spec.words(label).iter().any(|w| self.at_lex(w))
    }

    fn at_keyword(&self, label: &str) -> bool {
        self.peek().kind == Kind::Word && self.spec.spells(label, &self.peek().text)
    }

    fn at_separator(&self) -> bool {
        let t = self.peek();
        t.kind == Kind::Newline || (t.kind == Kind::Symbol && self.spec.spells("stmt.terminator", &t.text))
    }

    fn skip_separators(&mut self) {
        while !self.done() && self.at_separator() {
            self.take();
        }
    }

    fn need_sym(&mut self, s: &str, why: &str) -> Outcome<()> {
        if self.at_sym(s) {
            self.take();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", s, why, self.peek().text))
        }
    }

    fn need_word(&mut self, why: &str) -> Outcome<String> {
        if self.peek().kind == Kind::Word {
            Ok(self.take().text)
        } else {
            Err(format!("Expected identifier {}, got '{}'", why, self.peek().text))
        }
    }

    fn need_lex(&mut self, s: &str) -> Outcome<()> {
        if self.at_lex(s) {
            self.take();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", s, self.peek().text))
        }
    }

    fn need_closer(&mut self) -> Outcome<()> {
        if self.at_label("block.close") {
            self.take();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", self.spec.first("block.close").unwrap_or("end"), self.peek().text))
        }
    }

    // ---------- names ----------

    fn frame(&mut self) -> &mut Frame {
        self.frames.last_mut().expect("frame")
    }

    /// Where a name is read from.
    fn slot_of(&mut self, name: &str) -> Slot {
        let global = self.globals.slot(name);
        let frame = self.frames.last().expect("frame");
        let mut locals = Vec::new();
        if !frame.top {
            let in_blocks: Vec<usize> = frame.blocks.iter().flatten().copied().collect();
            for block in frame.blocks.iter().rev() {
                locals.extend(block.iter().rev().filter(|&&s| frame.slot_names[s] == name).copied());
            }
            if let Some(s) = frame.slot_names.iter().rposition(|n| n == name).filter(|s| !in_blocks.contains(s)) {
                locals.push(s);
            }
        }
        Slot { name: Rc::from(name), locals, global }
    }

    /// Where a name is written to: the innermost binding of the current
    /// block or program, made new if there is none.
    fn target(&mut self, name: &str) -> Slot {
        let global = self.globals.slot(name);
        let frame = self.frames.last_mut().expect("frame");
        if frame.top {
            let known = frame.slot_names.iter().any(|n| n == name);
            match frame.blocks.last_mut() {
                Some(block) if !known && !block.contains(&global) => block.push(global),
                None if !known => frame.slot_names.push(name.to_string()),
                _ => {}
            }
            return Slot { name: Rc::from(name), locals: Vec::new(), global };
        }
        let found = match frame.blocks.last() {
            Some(block) => block.iter().rev().find(|&&s| frame.slot_names[s] == name).copied(),
            None => {
                let in_blocks: Vec<usize> = frame.blocks.iter().flatten().copied().collect();
                frame.slot_names.iter().rposition(|n| n == name).filter(|s| !in_blocks.contains(s))
            }
        };
        let slot = found.unwrap_or_else(|| {
            frame.slot_names.push(name.to_string());
            let s = frame.slot_names.len() - 1;
            if let Some(block) = frame.blocks.last_mut() {
                block.push(s);
            }
            s
        });
        Slot { name: Rc::from(name), locals: vec![slot], global }
    }

    /// A slot no program names, for a loop bound or a spilled value.
    fn hidden(&mut self, what: &str) -> Slot {
        self.hidden += 1;
        let name = format!("#{}{}", what, self.hidden);
        self.target(&name)
    }

    fn load(&mut self, line: u32, name: &str) -> Node {
        let slot = self.slot_of(name);
        Node::new(line, Form::Load(slot))
    }

    fn assign(&mut self, line: u32, name: &str, value: Node) -> Node {
        let to = self.target(name);
        Node::new(line, Form::Assign { to, value: Box::new(value) })
    }

    fn literal(line: u32, v: Value) -> Node {
        Node::new(line, Form::Literal(v))
    }

    fn operate(line: u32, op: Op, args: Vec<Node>) -> Node {
        Node::new(line, Form::Operate { op, args })
    }

    // ---------- statements ----------

    fn body_until(&mut self, stops: &[String]) -> Outcome<Node> {
        let line = self.line();
        let mut items = Vec::new();
        self.skip_separators();
        while !self.done() && !stops.iter().any(|s| self.at_lex(s)) {
            items.push(self.statement()?);
            self.skip_separators();
        }
        Ok(Node::seq(line, items))
    }

    fn skip_intro(&mut self) {
        if self.at_label("block.intro") {
            self.take();
        }
    }

    /// A block after a statement head, in the language's layout.
    fn block(&mut self) -> Outcome<Node> {
        self.skip_intro();
        self.skip_separators();
        match self.spec.style {
            Style::Indent => {
                if self.peek().kind != Kind::Open {
                    return Err(format!("Expected an indented block, got '{}'", self.peek().text));
                }
                let line = self.take().line;
                let mut items = Vec::new();
                self.skip_separators();
                while self.peek().kind != Kind::Close && !self.done() {
                    items.push(self.statement()?);
                    self.skip_separators();
                }
                if self.peek().kind != Kind::Close {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.take();
                Ok(Node::seq(line, items))
            }
            Style::Brace => {
                let opens = self.spec.words("block.open");
                let k = opens
                    .iter()
                    .position(|o| self.at_lex(o))
                    .ok_or_else(|| format!("Expected '{}' to open a block, got '{}'", opens[0], self.peek().text))?;
                self.take();
                let close = self.spec.words("block.close")[k].clone();
                let body = self.body_until(std::slice::from_ref(&close))?;
                self.need_lex(&close)?;
                Ok(body)
            }
            Style::Keyword | Style::Postfix => {
                let closers = self.spec.words("block.close").to_vec();
                let body = self.body_until(&closers)?;
                self.need_closer()?;
                Ok(body)
            }
        }
    }

    fn statement(&mut self) -> Outcome<Node> {
        let spec = self.spec;
        if self.peek().kind == Kind::Word {
            if self.at_keyword("stmt.let") {
                return self.binding();
            }
            if self.at_keyword("stmt.if") {
                return self.branch();
            }
            if self.at_keyword("stmt.while") {
                return self.while_loop();
            }
            if self.at_keyword("stmt.until") {
                return self.until_loop();
            }
            if self.at_keyword("stmt.for") {
                return self.for_loop();
            }
            if self.at_keyword("stmt.return") {
                return self.return_stmt();
            }
            if self.at_keyword("stmt.break") {
                let line = self.take().line;
                return Ok(Node::new(line, Form::Leave { how: Exit::Break, value: None }));
            }
            if self.at_keyword("stmt.continue") {
                let line = self.take().line;
                return Ok(Node::new(line, Form::Leave { how: Exit::Continue, value: None }));
            }
            if self.at_keyword("stmt.function") {
                self.take();
                let name = self.need_word("after the function keyword")?;
                return self.function(name);
            }
            if self.at_keyword("stmt.pass") {
                let line = self.take().line;
                return Ok(Node::seq(line, Vec::new()));
            }
        }
        if spec.style == Style::Brace && self.at_label("block.open") {
            return self.bare_block();
        }
        self.assignment_or_expression()
    }

    fn bare_block(&mut self) -> Outcome<Node> {
        let line = self.line();
        self.frame().blocks.push(Vec::new());
        let body = self.block()?;
        let slots = self.frame().blocks.pop().expect("block");
        let top = self.frame().top;
        let forget = slots
            .into_iter()
            .map(|s| {
                if top {
                    Slot { name: Rc::from(self.globals.names[s].as_str()), locals: Vec::new(), global: s }
                } else {
                    let name = self.frames.last().unwrap().slot_names[s].clone();
                    let global = self.globals.slot(&name);
                    Slot { name: Rc::from(name.as_str()), locals: vec![s], global }
                }
            })
            .collect();
        Ok(Node::new(line, Form::Scope { forget, body: Box::new(body) }))
    }

    fn binding(&mut self) -> Outcome<Node> {
        if self.spec.flag("stmt.let.type_first") {
            return self.typed_declaration();
        }
        let line = self.take().line;
        if self.at_keyword("stmt.let.mutable") {
            self.take();
        }
        let name = self.need_word("after the binding keyword")?;
        if self.peek().kind == Kind::Symbol && self.spec.spells("stmt.let.annotation", &self.peek().text) {
            self.take();
            self.need_word("as a type name")?;
        }
        let value = if self.at_separator() || self.done() {
            Self::literal(line, Value::Nothing)
        } else {
            self.need_assign("in a binding")?;
            self.expression(0)?
        };
        Ok(self.assign(line, &name, value))
    }

    fn typed_declaration(&mut self) -> Outcome<Node> {
        let line = self.take().line;
        let name = self.need_word("after the type")?;
        if let Some(open) = self.spec.first("syntax.call.open") {
            if self.at_sym(open) {
                return self.function(name);
            }
        }
        let value = if self.at_separator() || self.done() {
            Self::literal(line, Value::Nothing)
        } else {
            self.need_assign("in a declaration")?;
            self.expression(0)?
        };
        Ok(self.assign(line, &name, value))
    }

    fn at_assign(&self) -> bool {
        self.peek().kind == Kind::Symbol && self.spec.spells("stmt.assign", &self.peek().text)
    }

    fn need_assign(&mut self, why: &str) -> Outcome<()> {
        if !self.spec.has("stmt.assign") {
            return Err("This language has no assignment operator".to_string());
        }
        if self.at_assign() {
            self.take();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", self.spec.first("stmt.assign").unwrap(), why, self.peek().text))
        }
    }

    fn branch(&mut self) -> Outcome<Node> {
        let keyword = self.spec.style == Style::Keyword;
        let line = self.take().line;
        let test = self.expression(0)?;
        let then = if keyword {
            self.skip_intro();
            let mut stops = self.spec.words("block.close").to_vec();
            stops.extend(self.spec.words("stmt.elif").iter().cloned());
            stops.extend(self.spec.words("stmt.else").iter().cloned());
            self.body_until(&stops)?
        } else {
            self.block()?
        };
        let mut look = 0;
        while self.ahead(look).kind == Kind::Newline
            || (self.ahead(look).kind == Kind::Symbol && self.spec.spells("stmt.terminator", &self.ahead(look).text))
        {
            look += 1;
        }
        let next = self.ahead(look);
        let elif = next.kind == Kind::Word && self.spec.spells("stmt.elif", &next.text);
        let else_ = next.kind == Kind::Word && self.spec.spells("stmt.else", &next.text);
        let otherwise = if elif {
            self.at += look;
            Some(self.branch()?)
        } else if else_ {
            self.at += look;
            self.take();
            if self.at_keyword("stmt.if") {
                Some(self.branch()?)
            } else if keyword {
                let closers = self.spec.words("block.close").to_vec();
                let body = self.body_until(&closers)?;
                self.need_closer()?;
                Some(body)
            } else {
                Some(self.block()?)
            }
        } else {
            if keyword {
                self.need_closer()?;
            }
            None
        };
        Ok(Node::new(line, Form::Branch { test: Box::new(test), then: Box::new(then), otherwise: otherwise.map(Box::new) }))
    }

    fn while_loop(&mut self) -> Outcome<Node> {
        let line = self.take().line;
        let test = self.expression(0)?;
        let body = self.block()?;
        Ok(Node::new(line, Form::Loop { test: Box::new(test), body: Box::new(body), step: None }))
    }

    /// `until cond block`: the body first, stopping once the condition holds.
    fn until_loop(&mut self) -> Outcome<Node> {
        let line = self.take().line;
        let test = self.expression(0)?;
        let body = self.block()?;
        Ok(until_node(line, test, body))
    }

    fn for_loop(&mut self) -> Outcome<Node> {
        let spec = self.spec;
        let line = self.take().line;
        let var = self.need_word("as the loop variable")?;
        if !self.at_keyword("stmt.for.in") {
            return Err(format!("Expected '{}' after for loop variable, got: {}", spec.first("stmt.for.in").unwrap_or("in"), self.peek().text));
        }
        self.take();
        let range_call = self.peek().kind == Kind::Word
            && spec.natives.get(&self.peek().text) == Some(&Native::Range)
            && spec.first("syntax.call.open").map_or(false, |o| self.ahead(1).kind == Kind::Symbol && self.ahead(1).text == o);
        let (start, end) = if range_call {
            self.take();
            self.take();
            let start = self.expression(0)?;
            if let Some(sep) = spec.first("syntax.call.separator") {
                self.need_sym(sep, "between the range bounds")?;
            }
            let end = self.expression(0)?;
            self.need_sym(spec.first("syntax.call.close").unwrap(), "after the range")?;
            (start, end)
        } else {
            let tier = spec.words("op.range").iter().filter_map(|r| spec.syntax_tier.get(r)).min().copied().unwrap_or(0);
            let start = self.expression(tier + 1)?;
            if !(self.peek().kind == Kind::Symbol && spec.spells("op.range", &self.peek().text)) {
                return Err("A for loop needs a range: start..end".to_string());
            }
            self.take();
            let end = self.expression(tier + 1)?;
            (start, end)
        };
        self.target(&var);
        let body = self.block()?;
        Ok(self.counted_loop(line, &var, start, end, body))
    }

    /// `for` as a counted loop: the bound in a hidden slot, the variable
    /// stepped by one after each pass.
    fn counted_loop(&mut self, line: u32, var: &str, start: Node, end: Node, body: Node) -> Node {
        let limit = self.hidden("end");
        let bind_limit = Node::new(line, Form::Assign { to: limit.clone(), value: Box::new(end) });
        let bind_var = self.assign(line, var, start);
        let test = Self::operate(line, Op::Lt, vec![self.load(line, var), Node::new(line, Form::Load(limit))]);
        let next = Self::operate(line, Op::Add, vec![self.load(line, var), Self::literal(line, Value::Small(1))]);
        let step = self.assign(line, var, next);
        Node::seq(line, vec![
            bind_limit,
            bind_var,
            Node::new(line, Form::Loop { test: Box::new(test), body: Box::new(body), step: Some(Box::new(step)) }),
        ])
    }

    fn return_stmt(&mut self) -> Outcome<Node> {
        let line = self.take().line;
        let value = if self.at_separator() || self.done() || self.at_label("block.close") {
            None
        } else {
            Some(Box::new(self.expression(0)?))
        };
        Ok(Node::new(line, Form::Leave { how: Exit::Return, value }))
    }

    /// From the parameter list on, then the body; the definition becomes a
    /// binding of a program value.
    fn function(&mut self, name: String) -> Outcome<Node> {
        let spec = self.spec;
        let line = self.line();
        let open = spec.first("syntax.call.open").ok_or_else(|| "This language has no call syntax".to_string())?;
        let close = spec.first("syntax.call.close").unwrap();
        self.need_sym(open, "after function name")?;
        let type_first = spec.flag("stmt.let.type_first");
        let mut params = Vec::new();
        while !self.at_sym(close) && !self.done() {
            if type_first {
                let word = self.need_word("as a parameter type")?;
                if !spec.spells("stmt.let", &word) {
                    return Err(format!("'{}' is not a type word", word));
                }
                if self.peek().kind == Kind::Word {
                    params.push(self.take().text);
                }
            } else {
                params.push(self.need_word("as a parameter name")?);
                if self.peek().kind == Kind::Symbol && spec.spells("stmt.let.annotation", &self.peek().text) {
                    self.take();
                    self.need_word("as a type name")?;
                }
            }
            if let Some(sep) = spec.first("syntax.call.separator") {
                if self.at_sym(sep) {
                    self.take();
                }
            }
            if self.peek().kind == Kind::Symbol && spec.spells("stmt.terminator", &self.peek().text) {
                self.take();
            }
        }
        self.need_sym(close, "after parameters")?;
        if self.peek().kind == Kind::Symbol && spec.spells("stmt.function.returns", &self.peek().text) {
            self.take();
            self.need_word("as a return type")?;
        }
        let declared = self.peek().kind == Kind::Symbol && spec.spells("stmt.terminator", &self.peek().text);
        let param_slots: Vec<usize> = (0..params.len()).collect();
        self.frames.push(Frame { top: false, slot_names: params.clone(), blocks: Vec::new(), params: Vec::new(), param_slots, postfix_program: false });
        let mut prelude = Vec::new();
        if declared {
            loop {
                self.skip_separators();
                if !type_first && self.at_keyword("stmt.let") {
                    prelude.push(self.binding()?);
                } else {
                    break;
                }
            }
        }
        let block = self.block()?;
        let body = if prelude.is_empty() {
            block
        } else {
            prelude.push(block);
            Node::seq(line, prelude)
        };
        let frame = self.frames.pop().expect("frame");
        let program = Program { name: name.clone(), params, param_slots: frame.param_slots, slot_names: frame.slot_names, body };
        let value = Self::literal(line, Value::Routine(Rc::new(program)));
        Ok(self.assign(line, &name, value))
    }

    fn assignment_or_expression(&mut self) -> Outcome<Node> {
        let expr = self.expression(0)?;
        if !self.at_assign() {
            return Ok(expr);
        }
        let assign = self.take();
        let line = assign.line;
        let value = self.expression(0)?;
        match expr.form {
            Form::Load(slot) => Ok(self.assign(line, &slot.name, value)),
            Form::Operate { op: Op::Index, mut args } if args.len() == 2 => {
                let index = args.pop().unwrap();
                match args.pop().unwrap().form {
                    Form::Load(slot) => {
                        let to = self.slot_of(&slot.name);
                        Ok(Node::new(line, Form::AssignIndex { to, index: Box::new(index), value: Box::new(value) }))
                    }
                    _ => Err("Invalid assignment target".to_string()),
                }
            }
            _ => Err(format!("Invalid assignment target before '{}'", assign.text)),
        }
    }

    // ---------- expressions ----------

    fn expression(&mut self, floor: u32) -> Outcome<Node> {
        let spec = self.spec;
        let mut left = self.prefix()?;
        loop {
            let tok = self.peek();
            if tok.kind != Kind::Symbol && tok.kind != Kind::Word {
                break;
            }
            let text = tok.text.clone();
            let line = tok.line;
            if spec.spells("op.pipe", &text) {
                if spec.syntax_tier.get(&text).copied().unwrap_or(0) < floor {
                    break;
                }
                self.take();
                left = self.pipe(line, left)?;
                continue;
            }
            let Some(op) = spec.infix.get(&text).copied() else { break };
            if op.tier < floor {
                break;
            }
            self.take();
            let right = self.expression(if op.right { op.tier } else { op.tier + 1 })?;
            left = Self::operate(line, op.op, vec![left, right]);
        }
        Ok(left)
    }

    /// The value on the left becomes the first argument of the call on the
    /// right; a bare name is a call with no other arguments.
    fn pipe(&mut self, line: u32, left: Node) -> Outcome<Node> {
        let name = self.need_word("after the pipe")?;
        let mut args = vec![left];
        if let Some(open) = self.spec.first("syntax.call.open") {
            if self.at_sym(open) {
                self.take();
                args.extend(self.arguments("syntax.call.close", "syntax.call.separator")?);
            }
        }
        self.call(line, &name, args)
    }

    fn prefix(&mut self) -> Outcome<Node> {
        let spec = self.spec;
        let tok = self.peek().clone();
        let line = tok.line;
        if matches!(tok.kind, Kind::Symbol | Kind::Word) {
            if let Some(op) = spec.prefix.get(&tok.text).copied() {
                self.take();
                let operand = self.expression(op.tier)?;
                return Ok(Self::operate(line, op.op, vec![operand]));
            }
        }
        let node = match tok.kind {
            Kind::Number => {
                self.take();
                Self::literal(line, number_literal(&tok.text, spec)?)
            }
            Kind::Str => {
                self.take();
                Self::literal(line, Value::from_text(&tok.text))
            }
            Kind::Word => {
                self.take();
                if spec.spells("literal.true", &tok.text) {
                    Self::literal(line, Value::Truth(true))
                } else if spec.spells("literal.false", &tok.text) {
                    Self::literal(line, Value::Truth(false))
                } else if spec.spells("literal.null", &tok.text) {
                    Self::literal(line, Value::Nothing)
                } else if spec.first("syntax.call.open").map_or(false, |o| self.at_sym(o)) {
                    self.take();
                    let args = self.arguments("syntax.call.close", "syntax.call.separator")?;
                    self.call(line, &tok.text, args)?
                } else {
                    self.load(line, &tok.text)
                }
            }
            Kind::Symbol => {
                if spec.first("syntax.group.open") == Some(tok.text.as_str()) {
                    self.take();
                    let inner = self.expression(0)?;
                    self.need_sym(spec.first("syntax.group.close").unwrap(), "to close a group")?;
                    inner
                } else if spec.first("syntax.array.open") == Some(tok.text.as_str()) {
                    self.take();
                    let items = self.arguments("syntax.array.close", "syntax.array.separator")?;
                    Self::operate(line, Op::Array, items)
                } else {
                    return Err(format!("Unexpected token: {}", tok.text));
                }
            }
            _ => return Err("Expected an expression".to_string()),
        };
        self.indexing(node)
    }

    fn indexing(&mut self, mut node: Node) -> Outcome<Node> {
        let (Some(open), Some(close)) = (self.spec.first("op.index.open"), self.spec.first("op.index.close")) else {
            return Ok(node);
        };
        while self.at_sym(open) {
            let line = self.take().line;
            let index = self.expression(0)?;
            self.need_sym(close, "after array index")?;
            node = Self::operate(line, Op::Index, vec![node, index]);
        }
        Ok(node)
    }

    /// Expressions up to the closing bracket, which is consumed. An
    /// argument label (Swift's `n:`) is dropped.
    fn arguments(&mut self, close_label: &str, sep_label: &str) -> Outcome<Vec<Node>> {
        let close = self.spec.first(close_label).unwrap().to_string();
        let sep = self.spec.first(sep_label).map(str::to_string);
        let mut items = Vec::new();
        while !self.at_sym(&close) {
            if self.done() {
                return Err(format!("Expected '{}'", close));
            }
            if self.peek().kind == Kind::Word && self.ahead(1).kind == Kind::Symbol && self.spec.spells("syntax.call.label", &self.ahead(1).text) {
                self.at += 2;
            }
            items.push(self.expression(0)?);
            if let Some(s) = &sep {
                if self.at_sym(s) {
                    self.take();
                }
            }
        }
        self.take();
        Ok(items)
    }

    /// A call by name: a builtin of the definition, or the program bound to the name.
    fn call(&mut self, line: u32, name: &str, args: Vec<Node>) -> Outcome<Node> {
        let callee = match self.spec.natives.get(name).copied() {
            Some(native @ (Native::Push | Native::Put)) => {
                if !matches!(args.first().map(|a| &a.form), Some(Form::Load(_))) {
                    return Err(format!("First argument to {}() must be an array variable name", name));
                }
                Callee::Native(native, Rc::from(name))
            }
            Some(native) => Callee::Native(native, Rc::from(name)),
            None => Callee::Named(self.slot_of(name)),
        };
        Ok(Node::new(line, Form::Call { callee, args }))
    }

    // ---------- postfix ----------

    /// Words up to one of `stops` or the end, read with a symbolic stack.
    /// Returns the statements and the nodes left on the stack.
    fn postfix_body(&mut self, stops: &[String]) -> Outcome<(Vec<Node>, Vec<Node>)> {
        let mut stmts: Vec<Node> = Vec::new();
        let mut stack: Vec<Node> = Vec::new();
        loop {
            self.skip_separators();
            if self.done() || stops.iter().any(|s| self.at_lex(s)) {
                return Ok((stmts, stack));
            }
            let tok = self.take();
            match tok.kind {
                Kind::Number => stack.push(Self::literal(tok.line, number_literal(&tok.text, self.spec)?)),
                Kind::Str => stack.push(Self::literal(tok.line, Value::from_text(&tok.text))),
                Kind::Name => {
                    self.skip_separators();
                    let taker = self.take();
                    if !matches!(taker.kind, Kind::Word | Kind::Symbol) {
                        return Err(format!("The name '{}' must be followed by a word that takes it", tok.text));
                    }
                    self.named_word(&taker, &tok.text, &mut stmts, &mut stack)?;
                }
                Kind::Word | Kind::Symbol => self.postfix_word(&tok, &mut stmts, &mut stack)?,
                _ => unreachable!("separators are skipped"),
            }
        }
    }

    /// Pop a node; from an empty stack inside a program, a new parameter.
    fn pop(&mut self, line: u32, stack: &mut Vec<Node>) -> Outcome<Node> {
        if let Some(node) = stack.pop() {
            return Ok(node);
        }
        let frame = self.frames.last_mut().expect("frame");
        if !frame.postfix_program {
            if self.strict {
                return Err(format!("Stack underflow (line {})", line));
            }
            return Ok(Self::literal(line, Value::Nothing));
        }
        let name = format!("p{}", frame.params.len() + 1);
        frame.params.push(name.clone());
        frame.slot_names.push(name.clone());
        frame.param_slots.push(frame.slot_names.len() - 1);
        Ok(self.load(line, &name))
    }

    /// Move every node with an effect off the stack into a hidden slot, so
    /// the statement about to be emitted runs after them.
    fn spill(&mut self, line: u32, stmts: &mut Vec<Node>, stack: &mut Vec<Node>) {
        for node in stack.iter_mut() {
            if !pure(node) {
                let slot = self.hidden("value");
                let value = std::mem::replace(node, Node::new(line, Form::Load(slot.clone())));
                stmts.push(Node::new(line, Form::Assign { to: slot, value: Box::new(value) }));
            }
        }
    }

    /// A loop body leaves nothing on the stack; while settling, what it
    /// leaves runs for its effects.
    fn loop_body(&mut self, line: u32, body: Node, left: Option<Node>) -> Outcome<Node> {
        match left {
            None => Ok(body),
            Some(_) if self.strict => Err(format!("A loop body may not leave a value on the stack (line {})", line)),
            Some(node) if pure(&node) => Ok(body),
            Some(node) => Ok(append(body, node)),
        }
    }

    fn need_intro(&mut self) -> Outcome<()> {
        if self.at_label("block.intro") {
            self.take();
            Ok(())
        } else {
            Err(format!("Expected '{}', got '{}'", self.spec.first("block.intro").unwrap_or("repeat"), self.peek().text))
        }
    }

    /// A body as one node: its statements, then whatever it left, which
    /// must be nothing or one value.
    fn postfix_block(&mut self, stops: &[String]) -> Outcome<(Node, Option<Node>)> {
        let line = self.line();
        let (mut stmts, mut rest) = self.postfix_body(stops)?;
        let leaves = match rest.len() {
            0 => None,
            1 => rest.pop(),
            n if self.strict => return Err(format!("A body may leave one value on the stack, this one leaves {} (line {})", n, line)),
            _ => {
                let last = rest.pop();
                stmts.extend(rest.into_iter().filter(|n| !pure(n)));
                last
            }
        };
        let returns = stmts.last().map_or(false, |s| matches!(s.form, Form::Leave { .. }));
        if returns && leaves.is_some() {
            stmts.push(leaves.unwrap());
            return Ok((Node::seq(line, stmts), None));
        }
        Ok((Node::seq(line, stmts), leaves))
    }

    fn named_word(&mut self, taker: &Token, name: &str, stmts: &mut Vec<Node>, stack: &mut Vec<Node>) -> Outcome<()> {
        let spec = self.spec;
        let word = taker.text.as_str();
        let line = taker.line;
        if spec.spells("stmt.assign", word) || spec.spells("stmt.let", word) {
            let value = self.pop(line, stack)?;
            self.spill(line, stmts, stack);
            // A program bound by name: remember what it takes and leaves.
            if let Form::Literal(Value::Routine(p)) = &value.form {
                let arity = Arity { takes: p.params.len(), leaves: leaves_value(&p.body) };
                self.found.insert(name.to_string(), arity);
                self.arities.insert(name.to_string(), arity);
            }
            let node = self.assign(line, name, value);
            stmts.push(node);
            return Ok(());
        }
        if spec.spells("stmt.for", word) {
            let end = self.pop(line, stack)?;
            let start = self.pop(line, stack)?;
            self.spill(line, stmts, stack);
            self.target(name);
            let closers = spec.words("block.close").to_vec();
            let (body, left) = self.postfix_block(&closers)?;
            self.need_closer()?;
            let body = self.loop_body(line, body, left)?;
            let node = self.counted_loop(line, name, start, end, body);
            stmts.push(node);
            return Ok(());
        }
        match spec.natives.get(word) {
            Some(Native::Push) => {
                let value = self.pop(line, stack)?;
                self.spill(line, stmts, stack);
                let target = self.load(line, name);
                stmts.push(Node::new(line, Form::Call { callee: Callee::Native(Native::Push, Rc::from(word)), args: vec![target, value] }));
                Ok(())
            }
            Some(Native::Put) => {
                let value = self.pop(line, stack)?;
                let index = self.pop(line, stack)?;
                self.spill(line, stmts, stack);
                let target = self.load(line, name);
                stmts.push(Node::new(line, Form::Call { callee: Callee::Native(Native::Put, Rc::from(word)), args: vec![target, index, value] }));
                Ok(())
            }
            _ => Err(format!("'{}' does not take a name, but '{}' was given", word, name)),
        }
    }

    fn postfix_word(&mut self, tok: &Token, stmts: &mut Vec<Node>, stack: &mut Vec<Node>) -> Outcome<()> {
        let spec = self.spec;
        let word = tok.text.as_str();
        let line = tok.line;
        let closers = spec.words("block.close").to_vec();
        if spec.spells("literal.true", word) {
            stack.push(Self::literal(line, Value::Truth(true)));
            return Ok(());
        }
        if spec.spells("literal.false", word) {
            stack.push(Self::literal(line, Value::Truth(false)));
            return Ok(());
        }
        if spec.spells("literal.null", word) {
            stack.push(Self::literal(line, Value::Nothing));
            return Ok(());
        }
        if spec.spells("stmt.if", word) {
            let test = self.pop(line, stack)?;
            self.spill(line, stmts, stack);
            let mut stops = closers.clone();
            stops.extend(spec.words("stmt.else").iter().cloned());
            let (then, then_left) = self.postfix_block(&stops)?;
            let (otherwise, else_left) = if self.at_label("stmt.else") {
                self.take();
                let (body, left) = self.postfix_block(&closers)?;
                (Some(body), left)
            } else {
                (None, None)
            };
            self.need_closer()?;
            let returns = |n: &Node| matches!(&n.form, Form::Sequence(items) if items.last().map_or(false, |s| matches!(s.form, Form::Leave { .. })));
            match (then_left, else_left) {
                (None, None) => {
                    stmts.push(Node::new(line, Form::Branch { test: Box::new(test), then: Box::new(then), otherwise: otherwise.map(Box::new) }));
                }
                (Some(a), Some(b)) => {
                    let then = append(then, a);
                    let otherwise = append(otherwise.unwrap(), b);
                    stack.push(Node::new(line, Form::Branch { test: Box::new(test), then: Box::new(then), otherwise: Some(Box::new(otherwise)) }));
                }
                (Some(a), None) if otherwise.as_ref().map_or(false, |o| returns(o)) => {
                    let then = append(then, a);
                    stack.push(Node::new(line, Form::Branch { test: Box::new(test), then: Box::new(then), otherwise: otherwise.map(Box::new) }));
                }
                (None, Some(b)) if returns(&then) => {
                    let otherwise = append(otherwise.unwrap(), b);
                    stack.push(Node::new(line, Form::Branch { test: Box::new(test), then: Box::new(then), otherwise: Some(Box::new(otherwise)) }));
                }
                _ if self.strict => return Err(format!("The branches of an if must leave the same number of values (line {})", line)),
                _ => {
                    stmts.push(Node::new(line, Form::Branch { test: Box::new(test), then: Box::new(then), otherwise: otherwise.map(Box::new) }));
                }
            }
            return Ok(());
        }
        if spec.spells("stmt.while", word) {
            self.spill(line, stmts, stack);
            let intros = spec.words("block.intro").to_vec();
            let (cond, test) = self.postfix_block(&intros)?;
            let test = test.ok_or_else(|| "A while condition must leave one value".to_string())?;
            self.need_intro()?;
            let (body, left) = self.postfix_block(&closers)?;
            self.need_closer()?;
            let body = self.loop_body(line, body, left)?;
            let test = append(cond, test);
            stmts.push(Node::new(line, Form::Loop { test: Box::new(test), body: Box::new(body), step: None }));
            return Ok(());
        }
        if spec.spells("stmt.until", word) {
            self.spill(line, stmts, stack);
            let intros = spec.words("block.intro").to_vec();
            let (body, left) = self.postfix_block(&intros)?;
            let body = self.loop_body(line, body, left)?;
            self.need_intro()?;
            let (cond, test) = self.postfix_block(&closers)?;
            let test = test.ok_or_else(|| "An until condition must leave one value".to_string())?;
            self.need_closer()?;
            stmts.push(until_node(line, append(cond, test), body));
            return Ok(());
        }
        if spec.spells("stmt.return", word) {
            let value = stack.pop().map(Box::new);
            self.spill(line, stmts, stack);
            stmts.push(Node::new(line, Form::Leave { how: Exit::Return, value }));
            return Ok(());
        }
        if spec.spells("stmt.break", word) {
            self.spill(line, stmts, stack);
            stmts.push(Node::new(line, Form::Leave { how: Exit::Break, value: None }));
            return Ok(());
        }
        if spec.spells("stmt.continue", word) {
            self.spill(line, stmts, stack);
            stmts.push(Node::new(line, Form::Leave { how: Exit::Continue, value: None }));
            return Ok(());
        }
        if spec.spells("stmt.for", word) {
            return Err(format!("'{}' needs a quoted name before it", word));
        }
        if let Some(k) = spec.words("stack.program.open").iter().position(|o| o == word) {
            let close = spec.words("stack.program.close")[k].clone();
            self.hidden += 1;
            let name = format!("<program{}>", self.hidden);
            self.frames.push(Frame { top: false, slot_names: Vec::new(), blocks: Vec::new(), params: Vec::new(), param_slots: Vec::new(), postfix_program: true });
            let (mut body, left) = self.postfix_block(std::slice::from_ref(&close))?;
            self.need_lex(&close)?;
            if let Some(value) = left {
                body = append_leave(body, value);
            }
            let frame = self.frames.pop().expect("frame");
            let mut params = frame.params;
            let mut param_slots = frame.param_slots;
            params.reverse();
            param_slots.reverse();
            let program = Program { name, params, param_slots, slot_names: frame.slot_names, body };
            stack.push(Self::literal(line, Value::Routine(Rc::new(program))));
            return Ok(());
        }
        if spec.first("syntax.array.open") == Some(word) {
            let close = spec.first("syntax.array.close").unwrap().to_string();
            let (inner_stmts, items) = self.postfix_body(std::slice::from_ref(&close))?;
            self.need_lex(&close)?;
            if !inner_stmts.is_empty() {
                self.spill(line, stmts, stack);
                stmts.extend(inner_stmts);
            }
            stack.push(Self::operate(line, Op::Array, items));
            return Ok(());
        }
        // Stack words: values with effects are spilled first so they run once, in order.
        if spec.spells("stack.dup", word) {
            let a = self.pop(line, stack)?;
            stack.push(a);
            self.spill(line, stmts, stack);
            let a = stack.pop().unwrap();
            let b = clone_pure(&a);
            stack.push(a);
            stack.push(b);
            return Ok(());
        }
        if spec.spells("stack.drop", word) {
            let a = self.pop(line, stack)?;
            if !pure(&a) {
                self.spill(line, stmts, stack);
                stmts.push(a);
            }
            return Ok(());
        }
        if spec.spells("stack.swap", word) {
            let b = self.pop(line, stack)?;
            let a = self.pop(line, stack)?;
            stack.push(a);
            stack.push(b);
            self.spill(line, stmts, stack);
            let b = stack.pop().unwrap();
            let a = stack.pop().unwrap();
            stack.push(b);
            stack.push(a);
            return Ok(());
        }
        if spec.spells("stack.over", word) {
            let b = self.pop(line, stack)?;
            let a = self.pop(line, stack)?;
            stack.push(a);
            stack.push(b);
            self.spill(line, stmts, stack);
            let a = clone_pure(&stack[stack.len() - 2]);
            stack.push(a);
            return Ok(());
        }
        if spec.spells("stack.rot", word) {
            let c = self.pop(line, stack)?;
            let b = self.pop(line, stack)?;
            let a = self.pop(line, stack)?;
            stack.push(a);
            stack.push(b);
            stack.push(c);
            self.spill(line, stmts, stack);
            let c = stack.pop().unwrap();
            let b = stack.pop().unwrap();
            let a = stack.pop().unwrap();
            stack.push(b);
            stack.push(c);
            stack.push(a);
            return Ok(());
        }
        if spec.spells("stack.eval", word) {
            let program = self.pop(line, stack)?;
            let arity = match &program.form {
                Form::Literal(Value::Routine(p)) => Arity { takes: p.params.len(), leaves: leaves_value(&p.body) },
                _ if self.strict => return Err(format!("eval needs a program written out where it is used (line {})", line)),
                _ => Arity { takes: 0, leaves: false },
            };
            let mut args = Vec::new();
            for _ in 0..arity.takes {
                args.push(self.pop(line, stack)?);
            }
            args.reverse();
            let call = Node::new(line, Form::Call { callee: Callee::Value(Box::new(program)), args });
            if arity.leaves {
                stack.push(call);
            } else {
                self.spill(line, stmts, stack);
                stmts.push(call);
            }
            return Ok(());
        }
        if let Some(op) = spec.infix.get(word).copied() {
            let b = self.pop(line, stack)?;
            let a = self.pop(line, stack)?;
            stack.push(Self::operate(line, op.op, vec![a, b]));
            return Ok(());
        }
        if let Some(op) = spec.prefix.get(word).copied() {
            let a = self.pop(line, stack)?;
            stack.push(Self::operate(line, op.op, vec![a]));
            return Ok(());
        }
        if let Some(native) = spec.natives.get(word).copied() {
            let (takes, leaves) = match native {
                Native::Push | Native::Put => return Err(format!("'{}' needs a quoted name before it", word)),
                Native::Extern | Native::Range => return Err(format!("'{}' has no postfix form", word)),
                Native::Emit | Native::Print | Native::Write | Native::Error => (1, false),
                Native::CharAt | Native::Get | Native::Real => (2, true),
                _ => (1, true),
            };
            let mut args = Vec::new();
            for _ in 0..takes {
                args.push(self.pop(line, stack)?);
            }
            args.reverse();
            let call = Node::new(line, Form::Call { callee: Callee::Native(native, Rc::from(word)), args });
            if leaves {
                stack.push(call);
            } else {
                self.spill(line, stmts, stack);
                stmts.push(call);
            }
            return Ok(());
        }
        if tok.kind != Kind::Word {
            return Err(format!("Unexpected '{}'", word));
        }
        // A bare word: a call to the program known under it, else a load.
        if let Some(arity) = self.arities.get(word).copied() {
            let mut args = Vec::new();
            for _ in 0..arity.takes {
                args.push(self.pop(line, stack)?);
            }
            args.reverse();
            let callee = Callee::Named(self.slot_of(word));
            let call = Node::new(line, Form::Call { callee, args });
            if arity.leaves {
                stack.push(call);
            } else {
                self.spill(line, stmts, stack);
                stmts.push(call);
            }
            return Ok(());
        }
        let node = self.load(line, word);
        stack.push(node);
        Ok(())
    }
}

/// `until cond body`: loop forever, the step breaking once cond holds.
fn until_node(line: u32, test: Node, body: Node) -> Node {
    let stop = Node::new(line, Form::Branch {
        test: Box::new(test),
        then: Box::new(Node::new(line, Form::Leave { how: Exit::Break, value: None })),
        otherwise: None,
    });
    Node::new(line, Form::Loop {
        test: Box::new(Node::new(line, Form::Literal(Value::Truth(true)))),
        body: Box::new(body),
        step: Some(Box::new(stop)),
    })
}

/// A sequence with one more item at the end.
fn append(seq: Node, item: Node) -> Node {
    match seq.form {
        Form::Sequence(mut items) => {
            items.push(item);
            Node::seq(seq.line, items)
        }
        other => Node::seq(seq.line, vec![Node::new(seq.line, other), item]),
    }
}

fn append_leave(seq: Node, value: Node) -> Node {
    let line = value.line;
    append(seq, Node::new(line, Form::Leave { how: Exit::Return, value: Some(Box::new(value)) }))
}

/// Whether a program body hands back a value: its last statement returns
/// one, or a branch of it does.
fn leaves_value(body: &Node) -> bool {
    match &body.form {
        Form::Sequence(items) => items.last().map_or(false, leaves_value),
        Form::Leave { how: Exit::Return, value } => value.is_some(),
        Form::Branch { then, otherwise, .. } => leaves_value(then) || otherwise.as_ref().map_or(false, |o| leaves_value(o)),
        _ => false,
    }
}

/// A copy of a node without effects.
fn clone_pure(node: &Node) -> Node {
    let form = match &node.form {
        Form::Literal(v) => Form::Literal(v.clone()),
        Form::Load(slot) => Form::Load(slot.clone()),
        Form::Operate { op, args } => Form::Operate { op: *op, args: args.iter().map(clone_pure).collect() },
        _ => unreachable!("impure nodes are spilled before they are copied"),
    };
    Node::new(node.line, form)
}

// ---------- number literals ----------

pub fn number_literal(text: &str, spec: &Spec) -> Outcome<Value> {
    if let Some(prefix) = spec.first("lexical.number.hex_prefix") {
        if let Some(digits) = text.strip_prefix(prefix) {
            return BigInt::parse_bytes(digits.as_bytes(), 16).map(Value::from_big).ok_or_else(|| format!("Invalid number: {}", text));
        }
    }
    let point = spec.glyph("lexical.number.decimal_point");
    if let Some(mark) = spec.glyph("lexical.number.base_marker").filter(|m| text.contains(*m)) {
        let (num, den) = in_base(text, mark, point, spec.glyph("lexical.number.exponent_marker"))?;
        return Ok(if den == BigInt::from(1) { Value::from_big(num) } else { numeric::value(num, den, Some(figures(text))) });
    }
    if let Some(p) = point {
        if let Some(dot) = text.find(p) {
            let (whole, frac) = (&text[..dot], &text[dot + p.len_utf8()..]);
            let scale = BigInt::from(10).pow(frac.len() as u32);
            let whole: BigInt = if whole.is_empty() { BigInt::from(0) } else { whole.parse().map_err(|_| format!("Invalid number: {}", text))? };
            let frac: BigInt = frac.parse().map_err(|_| format!("Invalid number: {}", text))?;
            return Ok(numeric::value(whole * &scale + frac, scale, Some(figures(text))));
        }
    }
    text.parse::<BigInt>().map(Value::from_big).map_err(|_| format!("Invalid number: {}", text))
}

fn figures(text: &str) -> usize {
    let digits: String = text.chars().filter(char::is_ascii_alphanumeric).collect();
    let zeros = digits.chars().take_while(|c| *c == '0').count();
    digits.len().saturating_sub(zeros).max(1).max(15)
}

fn in_base(text: &str, mark: char, point: Option<char>, exp: Option<char>) -> Outcome<(BigInt, BigInt)> {
    let at = text.find(mark).unwrap();
    let base: u32 = text[..at].parse().map_err(|_| format!("Invalid base in literal '{}': base must be decimal integer", text))?;
    if !(2..=36).contains(&base) {
        return Err(format!("Invalid base {}: must be between 2 and 36", base));
    }
    let rest = &text[at + mark.len_utf8()..];
    if rest.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits after '{}'", text, mark));
    }
    let (mantissa, exponent) = match exp.and_then(|e| rest.find(e).map(|p| (p, e))) {
        Some((p, e)) => (&rest[..p], Some(&rest[p + e.len_utf8()..])),
        None => (rest, None),
    };
    let (whole, frac) = match point.and_then(|d| mantissa.find(d).map(|p| (p, d))) {
        Some((p, d)) => (&mantissa[..p], Some(&mantissa[p + d.len_utf8()..])),
        None => (mantissa, None),
    };
    let digit_value = |s: &str| -> Outcome<BigInt> {
        let mut acc = BigInt::from(0);
        for c in s.chars() {
            let d = c.to_digit(36).ok_or_else(|| format!("Invalid base-N literal '{}': invalid digit '{}' for base {}", text, c, base))?;
            if d >= base {
                return Err(format!("Invalid base-N literal '{}': digit '{}' (value {}) is not valid in base {}", text, c, d, base));
            }
            acc = acc * base + d;
        }
        Ok(acc)
    };
    if whole.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits", text));
    }
    let mut num = digit_value(whole)?;
    let mut den = BigInt::from(1);
    match frac {
        Some(f) if !f.is_empty() => {
            den = BigInt::from(base).pow(f.len() as u32);
            num = num * &den + digit_value(f)?;
        }
        Some(_) => return Err(format!("Invalid base-N literal '{}': missing digits after '.'", text)),
        None => {}
    }
    if let Some(e) = exponent {
        if e.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits after exponent marker", text));
        }
        let e = digit_value(e)?.to_u32().ok_or_else(|| format!("Invalid base-N literal '{}': exponent too large", text))?;
        num *= BigInt::from(base).pow(e);
    }
    Ok((num, den))
}
