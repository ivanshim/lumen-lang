// Tokens to the four forms.
//
// Statements are told apart by the definition's words and expressions
// parsed by precedence over its tiers, as in any microcode kernel; the
// difference is what comes out. There is no branch form: `if` is a call
// of the `if` operation whose arms are program values. There is no loop
// form: a loop is a program bound to a hidden slot that calls itself.
// There is no sequence: `last` is called with the statements as its
// arguments. A postfix language is read with a symbolic stack.
//
// Every program value gets a frame at run time, so the reducer keeps a
// scope per program: functions and bare blocks own names, branch arms and
// loop bodies own none and resolve into the program around them.

use std::collections::HashMap;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::arith;
use crate::lexer::{Tk, Tok};
use crate::spec::{Layout, Spec};
use crate::tree::{Catch, Node, Op, Program, Slot, Target};
use crate::value::Value;

type Out<T> = Result<T, String>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Owns {
    /// A function: names assigned inside are its own; nothing outside is written.
    All,
    /// A bare block: names first assigned inside are its own.
    New,
    /// A branch arm or loop body: nothing; names belong to the program around.
    None,
}

struct Scope {
    owns: Owns,
    names: Vec<String>,
    params: Vec<String>,
    param_slots: Vec<usize>,
    postfix: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Arity {
    pub takes: usize,
    pub leaves: bool,
}

pub struct Reducer<'a> {
    spec: &'a Spec,
    toks: &'a [Tok],
    at: usize,
    scopes: Vec<Scope>,
    hidden: usize,
    pub assumed: HashMap<String, Arity>,
    pub found: HashMap<String, Arity>,
    strict: bool,
}

pub struct Reduced {
    pub program: Rc<Program>,
    pub global_names: Vec<String>,
    pub found: HashMap<String, Arity>,
}

/// What a run of postfix words is part of, which says where it stops.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Run {
    Block,
    Program,
    Line,
}

pub fn reduce(toks: &[Tok], spec: &Spec, seeded: &[String], assumed: HashMap<String, Arity>, strict: bool) -> Out<Reduced> {
    let top = Scope { owns: Owns::All, names: seeded.to_vec(), params: Vec::new(), param_slots: Vec::new(), postfix: false };
    let mut r = Reducer { spec, toks, at: 0, scopes: vec![top], hidden: 0, assumed, found: HashMap::new(), strict };
    let body = if spec.postfix {
        let (mut stmts, rest) = r.postfix_body(&[], Run::Block)?;
        if !r.done() {
            return Err(format!("Unexpected '{}'", r.peek().text));
        }
        stmts.extend(rest.into_iter().filter(|n| !pure(n)));
        seq(stmts)
    } else {
        let mut stmts = Vec::new();
        r.skip_ends();
        while !r.done() {
            stmts.push(r.statement()?);
            r.skip_ends();
        }
        seq(stmts)
    };
    let top = r.scopes.pop().unwrap();
    let program = Program { name: "<program>".into(), params: Vec::new(), param_slots: Vec::new(), names: top.names.clone(), catches: Catch::Nothing, body };
    Ok(Reduced { program: Rc::new(program), global_names: top.names, found: r.found })
}

// ---------- node builders

fn lit(v: Value) -> Node {
    Node::Literal(v)
}

fn call(op: Op, args: Vec<Node>) -> Node {
    let name: &str = match op {
        Op::Last => "last",
        Op::If => "if",
        Op::And => "and",
        Op::Or => "or",
        Op::Return => "return",
        Op::Break => "break",
        Op::Continue => "continue",
        _ => "",
    };
    let two = matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::DivReal | Op::Quot | Op::Rem | Op::Pow
        | Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Concat);
    if two && args.len() == 2 {
        let mut it = args.into_iter();
        let (a, b) = (it.next().unwrap(), it.next().unwrap());
        return Node::Binary { op, name: Rc::from(name), a: Box::new(a), b: Box::new(b) };
    }
    Node::Call(Target::Op(op, Rc::from(name)), args)
}

fn run(program: Node, args: Vec<Node>) -> Node {
    Node::Call(Target::Program(Box::new(program)), args)
}

/// Statements in order: one is itself, several are a call of `last`.
fn seq(mut items: Vec<Node>) -> Node {
    match items.len() {
        0 => lit(Value::Null),
        1 => items.pop().unwrap(),
        _ => call(Op::Last, items),
    }
}

fn pure(node: &Node) -> bool {
    match node {
        Node::Literal(_) | Node::Load(_) => true,
        Node::Binary { a, b, .. } => pure(a) && pure(b),
        Node::Call(Target::Op(op, _), args) => {
            !matches!(op, Op::Emit | Op::Print | Op::Write | Op::Error | Op::Extern | Op::Push | Op::Put | Op::Return | Op::Break | Op::Continue | Op::If | Op::And | Op::Or | Op::Last)
                && args.iter().all(pure)
        }
        _ => false,
    }
}

fn leaves_value(node: &Node) -> bool {
    match node {
        Node::Call(Target::Op(Op::Last, _), args) => args.last().map_or(false, leaves_value),
        Node::Call(Target::Op(Op::Return, _), args) => !args.is_empty(),
        Node::If { then, otherwise, .. } => leaves_value(then) || leaves_value(otherwise),
        _ => false,
    }
}

impl<'a> Reducer<'a> {
    // ---------- tokens

    fn peek(&self) -> &Tok {
        &self.toks[self.at.min(self.toks.len() - 1)]
    }

    fn look(&self, n: usize) -> &Tok {
        &self.toks[(self.at + n).min(self.toks.len() - 1)]
    }

    fn next(&mut self) -> Tok {
        let t = self.peek().clone();
        if self.at + 1 < self.toks.len() {
            self.at += 1;
        }
        t
    }

    fn done(&self) -> bool {
        self.peek().kind == Tk::End
    }

    fn sym(&self, s: &str) -> bool {
        self.peek().kind == Tk::Sym && self.peek().text == s
    }

    fn lex(&self, s: &str) -> bool {
        matches!(self.peek().kind, Tk::Sym | Tk::Word) && self.peek().text == s
    }

    fn at_any(&self, key: &str) -> bool {
        self.spec.list(key).iter().any(|w| self.lex(w))
    }

    fn keyword(&self, key: &str) -> bool {
        self.peek().kind == Tk::Word && self.spec.is(key, &self.peek().text)
    }

    fn at_end_of_statement(&self) -> bool {
        let t = self.peek();
        t.kind == Tk::Eol || (t.kind == Tk::Sym && self.spec.is("stmt.terminator", &t.text))
    }

    fn skip_ends(&mut self) {
        while !self.done() && self.at_end_of_statement() {
            self.next();
        }
    }

    fn want_sym(&mut self, s: &str, why: &str) -> Out<()> {
        if self.sym(s) {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", s, why, self.peek().text))
        }
    }

    fn want_word(&mut self, why: &str) -> Out<String> {
        if self.peek().kind == Tk::Word {
            Ok(self.next().text)
        } else {
            Err(format!("Expected identifier {}, got '{}'", why, self.peek().text))
        }
    }

    fn want_lex(&mut self, s: &str) -> Out<()> {
        if self.lex(s) {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", s, self.peek().text))
        }
    }

    fn want_closer(&mut self) -> Out<()> {
        if self.at_any("block.close") {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", self.spec.one("block.close").unwrap_or("end"), self.peek().text))
        }
    }

    // ---------- names

    fn global_index(&mut self, name: &str) -> usize {
        let top = &mut self.scopes[0];
        match top.names.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                top.names.push(name.to_string());
                top.names.len() - 1
            }
        }
    }

    /// The binding a read reaches: the nearest owner that has the name,
    /// stopping at a function; else the global.
    fn read_slot(&mut self, name: &str) -> Slot {
        let mut depth = 0;
        let mut found: Option<(usize, usize)> = None;
        for (i, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(index) = scope.names.iter().rposition(|n| n == name).filter(|_| scope.owns != Owns::None) {
                found = Some((depth, index));
                break;
            }
            if scope.owns == Owns::All && i != 0 {
                break;
            }
            depth += 1;
        }
        let global = self.global_index(name);
        match found {
            Some((depth, index)) if depth < self.scopes.len() - 1 => Slot { name: Rc::from(name), depth, index, global: Some(global) },
            Some((depth, index)) => Slot { name: Rc::from(name), depth, index, global: None },
            None => Slot { name: Rc::from(name), depth: self.scopes.len() - 1, index: global, global: None },
        }
    }

    /// The binding a write reaches: the nearest owner; a function or the
    /// top level makes the name if it has none, a block makes its own.
    fn write_slot(&mut self, name: &str) -> Slot {
        let mut depth = 0;
        let last = self.scopes.len() - 1;
        for i in (0..=last).rev() {
            let scope = &mut self.scopes[i];
            match scope.owns {
                Owns::None => {
                    depth += 1;
                    continue;
                }
                Owns::New | Owns::All => {
                    let index = match scope.names.iter().rposition(|n| n == name) {
                        Some(i) => i,
                        None => {
                            scope.names.push(name.to_string());
                            scope.names.len() - 1
                        }
                    };
                    return Slot { name: Rc::from(name), depth, index, global: None };
                }
            }
        }
        unreachable!("the top scope owns every name")
    }

    fn hidden(&mut self, what: &str) -> Slot {
        self.hidden += 1;
        let name = format!("#{}{}", what, self.hidden);
        self.write_slot(&name)
    }

    fn load(&mut self, name: &str) -> Node {
        Node::Load(self.read_slot(name))
    }

    fn assign(&mut self, name: &str, value: Node) -> Node {
        let slot = self.write_slot(name);
        // Cycle 4: `x = x + k` steps the binding in place.
        if let Node::Binary { op: Op::Add, a, b, .. } = &value {
            if let (Node::Load(read), Node::Literal(Value::Int(k))) = (a.as_ref(), b.as_ref()) {
                if read.name == slot.name && read.depth == slot.depth && read.index == slot.index && read.global == slot.global {
                    return Node::Step { slot, by: *k };
                }
            }
        }
        Node::Assign(slot, Box::new(value))
    }

    /// A program value: its body reduced in a scope of its own.
    fn program(&mut self, name: &str, owns: Owns, catches: Catch, params: Vec<String>, body: impl FnOnce(&mut Self) -> Out<Node>) -> Out<Node> {
        let param_slots = (0..params.len()).collect();
        self.scopes.push(Scope { owns, names: params.clone(), params: Vec::new(), param_slots, postfix: false });
        let body = body(self)?;
        let scope = self.scopes.pop().unwrap();
        Ok(lit(Value::Code(Rc::new(Program { name: name.to_string(), params, param_slots: scope.param_slots, names: scope.names, catches, body }))))
    }

    /// A branch arm or a loop body: a program that owns no names.
    fn arm(&mut self, catches: Catch, body: impl FnOnce(&mut Self) -> Out<Node>) -> Out<Node> {
        // Cycle 2: an arm that catches nothing is read in place, as part of
        // the program around it, not as a program of its own.
        if catches == Catch::Nothing {
            return body(self);
        }
        self.program("<arm>", Owns::None, catches, Vec::new(), body)
    }

    fn branch(&mut self, test: Node, then: Node, otherwise: Node) -> Node {
        Node::If { test: Box::new(test), then: Box::new(then), otherwise: Box::new(otherwise) }
    }

    /// `loop = « if test { body; step; loop() } »; loop()`. The test,
    /// the body and the step are read inside the programs they run in,
    /// so their names resolve to the right frames.
    fn looping<T, B, S>(&mut self, test: T, body: B, step: Option<S>) -> Out<Node>
    where
        T: FnOnce(&mut Self) -> Out<Node>,
        B: FnOnce(&mut Self) -> Out<Node>,
        S: FnOnce(&mut Self) -> Out<Node>,
    {
        let test = test(self)?;
        let body = body(self)?;
        let step = match step {
            Some(step) => Some(Box::new(step(self)?)),
            None => None,
        };
        Ok(Node::Loop { test: Box::new(test), body: Box::new(body), step, after: false })
    }

    /// `loop = « body; if test {} else { loop() } »; loop()`.
    fn until<B, T>(&mut self, body: B, test: T) -> Out<Node>
    where
        B: FnOnce(&mut Self) -> Out<Node>,
        T: FnOnce(&mut Self) -> Out<Node>,
    {
        // Read in source order: the test is written before the body.
        let test = test(self)?;
        let body = body(self)?;
        Ok(Node::Loop { test: Box::new(test), body: Box::new(body), step: None, after: true })
    }

    /// The counted loop: bound and variable set, then a loop stepping by one.
    fn counted<B>(&mut self, var: &str, start: Node, end: Node, body: B) -> Out<Node>
    where
        B: FnOnce(&mut Self) -> Out<Node>,
    {
        let limit = self.hidden("end");
        let limit_name = limit.name.to_string();
        let set_limit = Node::Assign(limit, Box::new(end));
        let set_var = self.assign(var, start);
        let (v1, v2) = (var.to_string(), var.to_string());
        let looped = self.looping(
            move |r| {
                let v = r.load(&v1);
                let l = r.load(&limit_name);
                Ok(call(Op::Lt, vec![v, l]))
            },
            body,
            Some(move |r: &mut Self| {
                let v = r.load(&v2);
                let next = call(Op::Add, vec![v, lit(Value::Int(1))]);
                Ok(r.assign(&v2, next))
            }),
        )?;
        Ok(seq(vec![set_limit, set_var, looped]))
    }

    // ---------- statements

    fn body_until(&mut self, stops: &[String]) -> Out<Node> {
        let mut items = Vec::new();
        self.skip_ends();
        while !self.done() && !stops.iter().any(|s| self.lex(s)) {
            items.push(self.statement()?);
            self.skip_ends();
        }
        Ok(seq(items))
    }

    fn skip_intro(&mut self) {
        if self.at_any("block.intro") {
            self.next();
        }
    }

    fn block(&mut self) -> Out<Node> {
        self.skip_intro();
        self.skip_ends();
        match self.spec.layout {
            Layout::Indent => {
                if self.peek().kind != Tk::BlockOpen {
                    return Err(format!("Expected an indented block, got '{}'", self.peek().text));
                }
                self.next();
                let mut items = Vec::new();
                self.skip_ends();
                while self.peek().kind != Tk::BlockClose && !self.done() {
                    items.push(self.statement()?);
                    self.skip_ends();
                }
                if self.peek().kind != Tk::BlockClose {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.next();
                Ok(seq(items))
            }
            Layout::Braces => {
                let opens = self.spec.list("block.open");
                let k = opens.iter().position(|o| self.lex(o)).ok_or_else(|| format!("Expected '{}' to open a block, got '{}'", opens[0], self.peek().text))?;
                self.next();
                let close = self.spec.list("block.close")[k].clone();
                let body = self.body_until(std::slice::from_ref(&close))?;
                self.want_lex(&close)?;
                Ok(body)
            }
            _ => {
                let closers = self.spec.list("block.close").to_vec();
                let body = self.body_until(&closers)?;
                self.want_closer()?;
                Ok(body)
            }
        }
    }

    /// A block in a scope of its own, as an arm program parsed in place.
    fn block_arm(&mut self, catches: Catch) -> Out<Node> {
        self.arm(catches, |r| r.block())
    }

    fn statement(&mut self) -> Out<Node> {
        if self.peek().kind == Tk::Word {
            if self.keyword("stmt.let") {
                return self.binding();
            }
            if self.keyword("stmt.if") {
                return self.if_statement();
            }
            if self.keyword("stmt.while") {
                self.next();
                let test_at = self.at;
                return self.looping(
                    move |r| {
                        r.at = test_at;
                        r.expression(0)
                    },
                    |r| r.block(),
                    None::<fn(&mut Self) -> Out<Node>>,
                );
            }
            if self.keyword("stmt.until") {
                self.next();
                let test_at = self.at;
                // The test is written before the body; find where it ends,
                // then read the body and, inside the loop, the test.
                let _ = self.expression(0)?;
                let body_at = self.at;
                return self.until(
                    move |r| {
                        r.at = body_at;
                        r.block()
                    },
                    move |r| {
                        let after = r.at;
                        r.at = test_at;
                        let test = r.expression(0)?;
                        r.at = after;
                        Ok(test)
                    },
                );
            }
            if self.keyword("stmt.for") {
                return self.for_statement();
            }
            if self.keyword("stmt.return") {
                self.next();
                let value = if self.at_end_of_statement() || self.done() || self.at_any("block.close") {
                    Vec::new()
                } else {
                    vec![self.expression(0)?]
                };
                return Ok(call(Op::Return, value));
            }
            if self.keyword("stmt.break") {
                self.next();
                return Ok(call(Op::Break, Vec::new()));
            }
            if self.keyword("stmt.continue") {
                self.next();
                return Ok(call(Op::Continue, Vec::new()));
            }
            if self.keyword("stmt.function") {
                self.next();
                let name = self.want_word("after the function keyword")?;
                return self.function(name);
            }
            if self.keyword("stmt.pass") {
                self.next();
                return Ok(lit(Value::Null));
            }
        }
        if self.spec.layout == Layout::Braces && self.at_any("block.open") {
            // A bare block: a program owning what it first binds, run at once.
            let program = self.program("<block>", Owns::New, Catch::Nothing, Vec::new(), |r| r.block())?;
            return Ok(run(program, Vec::new()));
        }
        self.assignment_or_expression()
    }

    fn if_statement(&mut self) -> Out<Node> {
        let keyword = self.spec.layout == Layout::Keyword;
        self.next();
        let test = self.expression(0)?;
        let then = if keyword {
            self.skip_intro();
            let mut stops = self.spec.list("block.close").to_vec();
            stops.extend(self.spec.list("stmt.elif").iter().cloned());
            stops.extend(self.spec.list("stmt.else").iter().cloned());
            self.arm(Catch::Nothing, |r| r.body_until(&stops))?
        } else {
            self.block_arm(Catch::Nothing)?
        };
        let mut ahead = 0;
        while self.look(ahead).kind == Tk::Eol || (self.look(ahead).kind == Tk::Sym && self.spec.is("stmt.terminator", &self.look(ahead).text)) {
            ahead += 1;
        }
        let next = self.look(ahead);
        let elif = next.kind == Tk::Word && self.spec.is("stmt.elif", &next.text);
        let else_ = next.kind == Tk::Word && self.spec.is("stmt.else", &next.text);
        let otherwise = if elif {
            self.at += ahead;
            self.arm(Catch::Nothing, |r| r.if_statement())?
        } else if else_ {
            self.at += ahead;
            self.next();
            if self.keyword("stmt.if") {
                self.arm(Catch::Nothing, |r| r.if_statement())?
            } else if keyword {
                let closers = self.spec.list("block.close").to_vec();
                let arm = self.arm(Catch::Nothing, |r| r.body_until(&closers))?;
                self.want_closer()?;
                arm
            } else {
                self.block_arm(Catch::Nothing)?
            }
        } else {
            if keyword {
                self.want_closer()?;
            }
            self.arm(Catch::Nothing, |_| Ok(lit(Value::Null)))?
        };
        Ok(self.branch(test, then, otherwise))
    }

    fn for_statement(&mut self) -> Out<Node> {
        let spec = self.spec;
        self.next();
        let var = self.want_word("as the loop variable")?;
        if !self.keyword("stmt.for.in") {
            return Err(format!("Expected '{}' after for loop variable, got: {}", spec.one("stmt.for.in").unwrap_or("in"), self.peek().text));
        }
        self.next();
        let ranged = self.peek().kind == Tk::Word
            && spec.builtins.get(&self.peek().text) == Some(&Op::Range)
            && spec.one("syntax.call.open").map_or(false, |o| self.look(1).kind == Tk::Sym && self.look(1).text == o);
        let (start, end) = if ranged {
            self.next();
            self.next();
            let start = self.expression(0)?;
            if let Some(sep) = spec.one("syntax.call.separator") {
                self.want_sym(sep, "between the range bounds")?;
            }
            let end = self.expression(0)?;
            self.want_sym(spec.one("syntax.call.close").unwrap(), "after the range")?;
            (start, end)
        } else {
            let tier = spec.list("op.range").iter().filter_map(|r| spec.syntax_tier.get(r)).min().copied().unwrap_or(0);
            let start = self.expression(tier + 1)?;
            if !(self.peek().kind == Tk::Sym && spec.is("op.range", &self.peek().text)) {
                return Err("A for loop needs a range: start..end".to_string());
            }
            self.next();
            let end = self.expression(tier + 1)?;
            (start, end)
        };
        self.write_slot(&var);
        self.counted(&var, start, end, |r| r.block())
    }

    fn binding(&mut self) -> Out<Node> {
        if self.spec.on("stmt.let.type_first") {
            self.next();
            let name = self.want_word("after the type")?;
            if let Some(open) = self.spec.one("syntax.call.open") {
                if self.sym(open) {
                    return self.function(name);
                }
            }
            let value = if self.at_end_of_statement() || self.done() {
                lit(Value::Null)
            } else {
                self.want_assign("in a declaration")?;
                self.expression(0)?
            };
            return Ok(self.assign(&name, value));
        }
        self.next();
        if self.keyword("stmt.let.mutable") {
            self.next();
        }
        let name = self.want_word("after the binding keyword")?;
        if self.peek().kind == Tk::Sym && self.spec.is("stmt.let.annotation", &self.peek().text) {
            self.next();
            self.want_word("as a type name")?;
        }
        let value = if self.at_end_of_statement() || self.done() {
            lit(Value::Null)
        } else {
            self.want_assign("in a binding")?;
            self.expression(0)?
        };
        Ok(self.assign(&name, value))
    }

    fn at_assign(&self) -> bool {
        self.peek().kind == Tk::Sym && self.spec.is("stmt.assign", &self.peek().text)
    }

    fn want_assign(&mut self, why: &str) -> Out<()> {
        if !self.spec.any("stmt.assign") {
            return Err("This language has no assignment operator".to_string());
        }
        if self.at_assign() {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", self.spec.one("stmt.assign").unwrap(), why, self.peek().text))
        }
    }

    fn function(&mut self, name: String) -> Out<Node> {
        let spec = self.spec;
        let open = spec.one("syntax.call.open").ok_or_else(|| "This language has no call syntax".to_string())?;
        let close = spec.one("syntax.call.close").unwrap();
        self.want_sym(open, "after function name")?;
        let typed = spec.on("stmt.let.type_first");
        let mut params = Vec::new();
        while !self.sym(close) && !self.done() {
            if typed {
                let t = self.want_word("as a parameter type")?;
                if !spec.is("stmt.let", &t) {
                    return Err(format!("'{}' is not a type word", t));
                }
                if self.peek().kind == Tk::Word {
                    params.push(self.next().text);
                }
            } else {
                params.push(self.want_word("as a parameter name")?);
                if self.peek().kind == Tk::Sym && spec.is("stmt.let.annotation", &self.peek().text) {
                    self.next();
                    self.want_word("as a type name")?;
                }
            }
            if let Some(sep) = spec.one("syntax.call.separator") {
                if self.sym(sep) {
                    self.next();
                }
            }
            if self.peek().kind == Tk::Sym && spec.is("stmt.terminator", &self.peek().text) {
                self.next();
            }
        }
        self.want_sym(close, "after parameters")?;
        if self.peek().kind == Tk::Sym && spec.is("stmt.function.returns", &self.peek().text) {
            self.next();
            self.want_word("as a return type")?;
        }
        let declared = self.peek().kind == Tk::Sym && spec.is("stmt.terminator", &self.peek().text);
        let program = self.program(&name, Owns::All, Catch::Return, params, |r| {
            let mut items = Vec::new();
            if declared {
                loop {
                    r.skip_ends();
                    if !typed && r.keyword("stmt.let") {
                        items.push(r.binding()?);
                    } else {
                        break;
                    }
                }
            }
            items.push(r.block()?);
            Ok(seq(items))
        })?;
        Ok(self.assign(&name, program))
    }

    fn assignment_or_expression(&mut self) -> Out<Node> {
        let expr = self.expression(0)?;
        if !self.at_assign() {
            return Ok(expr);
        }
        let assign = self.next();
        let value = self.expression(0)?;
        match expr {
            Node::Load(slot) => Ok(self.assign(&slot.name, value)),
            Node::Call(Target::Op(Op::Index, _), mut args) if args.len() == 2 => {
                let index = args.pop().unwrap();
                match args.pop().unwrap() {
                    Node::Load(slot) => {
                        let target = self.load(&slot.name);
                        Ok(call(Op::Put, vec![target, index, value]))
                    }
                    _ => Err("Invalid assignment target".to_string()),
                }
            }
            _ => Err(format!("Invalid assignment target before '{}'", assign.text)),
        }
    }

    // ---------- expressions

    fn expression(&mut self, floor: u32) -> Out<Node> {
        let spec = self.spec;
        let mut left = self.prefix()?;
        loop {
            let t = self.peek();
            if t.kind != Tk::Sym && t.kind != Tk::Word {
                break;
            }
            let text = t.text.clone();
            if spec.is("op.pipe", &text) {
                if spec.syntax_tier.get(&text).copied().unwrap_or(0) < floor {
                    break;
                }
                self.next();
                let name = self.want_word("after the pipe")?;
                let mut args = vec![left];
                if let Some(open) = spec.one("syntax.call.open") {
                    if self.sym(open) {
                        self.next();
                        args.extend(self.arguments("syntax.call.close", "syntax.call.separator")?);
                    }
                }
                left = self.call_named(&name, args)?;
                continue;
            }
            let Some(op) = spec.binary.get(&text).copied() else { break };
            if op.tier < floor {
                break;
            }
            self.next();
            let floor_right = if op.right { op.tier } else { op.tier + 1 };
            left = match op.op {
                // The right side is a program, run only when the left leaves it open.
                Op::And | Op::Or => {
                    let lazy = self.arm(Catch::Nothing, |r| r.expression(floor_right))?;
                    call(op.op, vec![left, lazy])
                }
                other => {
                    let right = self.expression(floor_right)?;
                    call(other, vec![left, right])
                }
            };
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Out<Node> {
        let spec = self.spec;
        let t = self.peek().clone();
        if matches!(t.kind, Tk::Sym | Tk::Word) {
            if let Some(op) = spec.unary.get(&t.text).copied() {
                self.next();
                let operand = self.expression(op.tier)?;
                return Ok(call(op.op, vec![operand]));
            }
        }
        let node = match t.kind {
            Tk::Number => {
                self.next();
                lit(number(&t.text, spec)?)
            }
            Tk::Text => {
                self.next();
                lit(Value::str(&t.text))
            }
            Tk::Word => {
                self.next();
                if spec.is("literal.true", &t.text) {
                    lit(Value::Bool(true))
                } else if spec.is("literal.false", &t.text) {
                    lit(Value::Bool(false))
                } else if spec.is("literal.null", &t.text) {
                    lit(Value::Null)
                } else if spec.one("syntax.call.open").map_or(false, |o| self.sym(o)) {
                    self.next();
                    let args = self.arguments("syntax.call.close", "syntax.call.separator")?;
                    self.call_named(&t.text, args)?
                } else {
                    self.load(&t.text)
                }
            }
            Tk::Sym => {
                if spec.one("syntax.group.open") == Some(t.text.as_str()) {
                    self.next();
                    let inner = self.expression(0)?;
                    self.want_sym(spec.one("syntax.group.close").unwrap(), "to close a group")?;
                    inner
                } else if spec.one("syntax.array.open") == Some(t.text.as_str()) {
                    self.next();
                    let items = self.arguments("syntax.array.close", "syntax.array.separator")?;
                    call(Op::Array, items)
                } else {
                    return Err(format!("Unexpected token: {}", t.text));
                }
            }
            _ => return Err("Expected an expression".to_string()),
        };
        self.indexed(node)
    }

    fn indexed(&mut self, mut node: Node) -> Out<Node> {
        let (Some(open), Some(close)) = (self.spec.one("op.index.open"), self.spec.one("op.index.close")) else { return Ok(node) };
        while self.sym(open) {
            self.next();
            let index = self.expression(0)?;
            self.want_sym(close, "after array index")?;
            node = call(Op::Index, vec![node, index]);
        }
        Ok(node)
    }

    fn arguments(&mut self, close_key: &str, sep_key: &str) -> Out<Vec<Node>> {
        let close = self.spec.one(close_key).unwrap().to_string();
        let sep = self.spec.one(sep_key).map(str::to_string);
        let mut items = Vec::new();
        while !self.sym(&close) {
            if self.done() {
                return Err(format!("Expected '{}'", close));
            }
            if self.peek().kind == Tk::Word && self.look(1).kind == Tk::Sym && self.spec.is("syntax.call.label", &self.look(1).text) {
                self.at += 2;
            }
            items.push(self.expression(0)?);
            if let Some(s) = &sep {
                if self.sym(s) {
                    self.next();
                }
            }
        }
        self.next();
        Ok(items)
    }

    fn call_named(&mut self, name: &str, args: Vec<Node>) -> Out<Node> {
        match self.spec.builtins.get(name).copied() {
            Some(op @ (Op::Push | Op::Put)) => {
                if !matches!(args.first(), Some(Node::Load(_))) {
                    return Err(format!("First argument to {}() must be an array variable name", name));
                }
                Ok(Node::Call(Target::Op(op, Rc::from(name)), args))
            }
            Some(op) => Ok(Node::Call(Target::Op(op, Rc::from(name)), args)),
            None => {
                let target = self.load(name);
                Ok(run(target, args))
            }
        }
    }

    // ---------- postfix

    /// Words up to a stop or the end, as statements and a symbolic stack.
    /// Where else they stop depends on what they are: a block's words at
    /// its end (a deeper indentation is an error), a program value's at
    /// its bracket (indentation passes), a line's at the line end.
    fn postfix_body(&mut self, stops: &[String], run: Run) -> Out<(Vec<Node>, Vec<Node>)> {
        let mut stmts = Vec::new();
        let mut stack = Vec::new();
        loop {
            if run == Run::Line {
                while self.peek().kind == Tk::Sym && self.spec.is("stmt.terminator", &self.peek().text) {
                    self.next();
                }
            } else {
                self.skip_ends();
            }
            if self.done() || stops.iter().any(|s| self.lex(s)) {
                return Ok((stmts, stack));
            }
            let mark = self.peek().kind;
            if matches!(mark, Tk::BlockOpen | Tk::BlockClose | Tk::Eol) {
                if run == Run::Program {
                    self.next();
                    continue;
                }
                if run == Run::Block && mark == Tk::BlockOpen {
                    return Err("Unexpected indentation".to_string());
                }
                return Ok((stmts, stack));
            }
            let t = self.next();
            match t.kind {
                Tk::Number => stack.push(lit(number(&t.text, self.spec)?)),
                Tk::Text => stack.push(lit(Value::str(&t.text))),
                Tk::Name => {
                    self.skip_ends();
                    let taker = self.next();
                    if !matches!(taker.kind, Tk::Word | Tk::Sym) {
                        return Err(format!("The name '{}' must be followed by a word that takes it", t.text));
                    }
                    self.named(&taker, &t.text, &mut stmts, &mut stack)?;
                }
                Tk::Word | Tk::Sym => self.word(&t, &mut stmts, &mut stack)?,
                _ => unreachable!(),
            }
        }
    }

    fn pop(&mut self, stack: &mut Vec<Node>) -> Out<Node> {
        if let Some(n) = stack.pop() {
            return Ok(n);
        }
        // The nearest postfix program takes one more parameter.
        let Some(i) = self.scopes.iter().rposition(|s| s.postfix) else {
            return if self.strict { Err("Stack underflow".to_string()) } else { Ok(lit(Value::Null)) };
        };
        let scope = &mut self.scopes[i];
        let name = format!("p{}", scope.params.len() + 1);
        scope.params.push(name.clone());
        scope.names.push(name.clone());
        scope.param_slots.push(scope.names.len() - 1);
        Ok(self.load(&name))
    }

    fn spill(&mut self, stmts: &mut Vec<Node>, stack: &mut Vec<Node>) {
        for node in stack.iter_mut() {
            if !pure(node) {
                let slot = self.hidden("value");
                let value = std::mem::replace(node, Node::Load(slot.clone()));
                stmts.push(Node::Assign(slot, Box::new(value)));
            }
        }
    }

    fn want_intro(&mut self) -> Out<()> {
        if self.at_any("block.intro") {
            self.next();
            Ok(())
        } else {
            Err(format!("Expected '{}', got '{}'", self.spec.one("block.intro").unwrap_or("repeat"), self.peek().text))
        }
    }

    /// The body a control word governs, in the block style: indented
    /// lines, a bracketed run, or words up to a closer, which is taken.
    fn governed(&mut self) -> Out<(Vec<Node>, Option<Node>)> {
        match self.spec.layout {
            Layout::Indent => {
                self.skip_ends();
                if self.peek().kind != Tk::BlockOpen {
                    return Err(format!("Expected an indented block, got '{}'", self.peek().text));
                }
                self.next();
                let body = self.postfix_block(&[], Run::Block)?;
                if self.peek().kind != Tk::BlockClose {
                    return Err("Expected the end of an indented block".to_string());
                }
                self.next();
                Ok(body)
            }
            Layout::Braces => {
                let opens = self.spec.list("block.open");
                let k = opens.iter().position(|o| self.lex(o)).ok_or_else(|| format!("Expected '{}' to open a block, got '{}'", opens[0], self.peek().text))?;
                self.next();
                let close = self.spec.list("block.close")[k].clone();
                let body = self.postfix_block(std::slice::from_ref(&close), Run::Block)?;
                self.want_lex(&close)?;
                Ok(body)
            }
            Layout::Keyword => {
                let closers = self.spec.list("block.close").to_vec();
                let body = self.postfix_block(&closers, Run::Block)?;
                self.want_closer()?;
                Ok(body)
            }
        }
    }

    /// A loop's condition, read again each pass: the words up to where the
    /// body begins by style, the line end, the opener, or the intro word.
    fn postfix_condition(&mut self) -> Out<(Vec<Node>, Option<Node>)> {
        match self.spec.layout {
            Layout::Indent => self.postfix_block(&[], Run::Line),
            Layout::Braces => {
                let opens = self.spec.list("block.open").to_vec();
                self.postfix_block(&opens, Run::Block)
            }
            Layout::Keyword => {
                let intros = self.spec.list("block.intro").to_vec();
                let test = self.postfix_block(&intros, Run::Block)?;
                self.want_intro()?;
                Ok(test)
            }
        }
    }

    /// Step onto an else word if one follows past line ends.
    fn take_else(&mut self) -> bool {
        let mut ahead = 0;
        loop {
            let t = &self.toks[(self.at + ahead).min(self.toks.len() - 1)];
            let separator = t.kind == Tk::Eol || (t.kind == Tk::Sym && self.spec.is("stmt.terminator", &t.text));
            if !separator {
                if t.kind == Tk::Word && self.spec.is("stmt.else", &t.text) {
                    self.at += ahead + 1;
                    return true;
                }
                return false;
            }
            ahead += 1;
        }
    }

    /// A body: its statements and what it leaves, at most one value.
    fn postfix_block(&mut self, stops: &[String], run: Run) -> Out<(Vec<Node>, Option<Node>)> {
        let (mut stmts, mut rest) = self.postfix_body(stops, run)?;
        let left = match rest.len() {
            0 => None,
            1 => rest.pop(),
            n if self.strict => return Err(format!("A body may leave one value on the stack, this one leaves {}", n)),
            _ => {
                let last = rest.pop();
                stmts.extend(rest.into_iter().filter(|n| !pure(n)));
                last
            }
        };
        let returns = matches!(stmts.last(), Some(Node::Call(Target::Op(Op::Return | Op::Break | Op::Continue, _), _)));
        if returns && left.is_some() {
            stmts.push(left.unwrap());
            return Ok((stmts, None));
        }
        Ok((stmts, left))
    }

    fn loop_body(&mut self, stmts: Vec<Node>, left: Option<Node>) -> Out<Node> {
        match left {
            None => Ok(seq(stmts)),
            Some(_) if self.strict => Err("A loop body may not leave a value on the stack".to_string()),
            Some(n) if pure(&n) => Ok(seq(stmts)),
            Some(n) => {
                let mut stmts = stmts;
                stmts.push(n);
                Ok(seq(stmts))
            }
        }
    }

    fn named(&mut self, taker: &Tok, name: &str, stmts: &mut Vec<Node>, stack: &mut Vec<Node>) -> Out<()> {
        let spec = self.spec;
        let word = taker.text.as_str();
        if spec.is("stmt.assign", word) || spec.is("stmt.let", word) {
            let value = self.pop(stack)?;
            self.spill(stmts, stack);
            if let Node::Literal(Value::Code(p)) = &value {
                let arity = Arity { takes: p.params.len(), leaves: leaves_value(&p.body) };
                self.found.insert(name.to_string(), arity);
                self.assumed.insert(name.to_string(), arity);
            }
            let node = self.assign(name, value);
            stmts.push(node);
            return Ok(());
        }
        if spec.is("stmt.for", word) {
            let end = self.pop(stack)?;
            let start = self.pop(stack)?;
            self.spill(stmts, stack);
            self.write_slot(name);
            let node = self.counted(name, start, end, |r| {
                let (s, left) = r.governed()?;
                r.loop_body(s, left)
            })?;
            stmts.push(node);
            return Ok(());
        }
        match spec.builtins.get(word).copied() {
            Some(Op::Push) => {
                let value = self.pop(stack)?;
                self.spill(stmts, stack);
                let target = self.load(name);
                stmts.push(Node::Call(Target::Op(Op::Push, Rc::from(word)), vec![target, value]));
                Ok(())
            }
            Some(Op::Put) => {
                let value = self.pop(stack)?;
                let index = self.pop(stack)?;
                self.spill(stmts, stack);
                let target = self.load(name);
                stmts.push(Node::Call(Target::Op(Op::Put, Rc::from(word)), vec![target, index, value]));
                Ok(())
            }
            _ => Err(format!("'{}' does not take a name, but '{}' was given", word, name)),
        }
    }

    fn word(&mut self, t: &Tok, stmts: &mut Vec<Node>, stack: &mut Vec<Node>) -> Out<()> {
        let spec = self.spec;
        let w = t.text.as_str();
        let closers = spec.list("block.close").to_vec();
        for (key, v) in [("literal.true", Value::Bool(true)), ("literal.false", Value::Bool(false)), ("literal.null", Value::Null)] {
            if spec.is(key, w) {
                stack.push(lit(v));
                return Ok(());
            }
        }
        if spec.is("stmt.if", w) {
            let test = self.pop(stack)?;
            self.spill(stmts, stack);
            // In the keyword style one closer ends both arms; in the
            // others each arm is a governed block.
            let keyword = spec.layout == Layout::Keyword;
            let mut stops = closers.clone();
            stops.extend(spec.list("stmt.else").iter().cloned());
            // Each arm is read inside its own program; what it leaves is its value.
            let mut then_left = false;
            let mut then_returns = false;
            let then = self.arm(Catch::Nothing, |r| {
                let (mut s, left) = if keyword { r.postfix_block(&stops, Run::Block)? } else { r.governed()? };
                then_returns = matches!(s.last(), Some(Node::Call(Target::Op(Op::Return | Op::Break | Op::Continue, _), _)));
                then_left = left.is_some();
                s.extend(left);
                Ok(seq(s))
            })?;
            let mut else_left = false;
            let mut else_returns = false;
            let has_else = if keyword { self.at_any("stmt.else") && { self.next(); true } } else { self.take_else() };
            let otherwise = if has_else {
                self.arm(Catch::Nothing, |r| {
                    let (mut s, left) = if keyword { r.postfix_block(&closers, Run::Block)? } else { r.governed()? };
                    else_returns = matches!(s.last(), Some(Node::Call(Target::Op(Op::Return | Op::Break | Op::Continue, _), _)));
                    else_left = left.is_some();
                    s.extend(left);
                    Ok(seq(s))
                })?
            } else {
                self.arm(Catch::Nothing, |_| Ok(lit(Value::Null)))?
            };
            if keyword {
                self.want_closer()?;
            }
            let as_value = match (then_left, else_left) {
                (false, false) => false,
                (true, true) => true,
                (true, false) if else_returns => true,
                (false, true) if then_returns => true,
                _ if self.strict => return Err("The branches of an if must leave the same number of values".to_string()),
                _ => false,
            };
            let node = self.branch(test, then, otherwise);
            if as_value {
                stack.push(node);
            } else {
                stmts.push(node);
            }
            return Ok(());
        }
        if spec.is("stmt.while", w) {
            self.spill(stmts, stack);
            let node = self.looping(
                |r| {
                    let (s, left) = r.postfix_condition()?;
                    let mut items = s;
                    items.push(left.ok_or_else(|| "A while condition must leave one value".to_string())?);
                    Ok(seq(items))
                },
                |r| {
                    let (s, left) = r.governed()?;
                    r.loop_body(s, left)
                },
                None::<fn(&mut Self) -> Out<Node>>,
            )?;
            stmts.push(node);
            return Ok(());
        }
        if spec.is("stmt.until", w) {
            // Written head first as in Lumen, the condition tested after the body.
            self.spill(stmts, stack);
            let node = self.until(
                |r| {
                    let (s, left) = r.governed()?;
                    r.loop_body(s, left)
                },
                |r| {
                    let (s, left) = r.postfix_condition()?;
                    let mut items = s;
                    items.push(left.ok_or_else(|| "An until condition must leave one value".to_string())?);
                    Ok(seq(items))
                },
            )?;
            stmts.push(node);
            return Ok(());
        }
        if spec.is("stmt.return", w) {
            let value: Vec<Node> = stack.pop().into_iter().collect();
            self.spill(stmts, stack);
            stmts.push(call(Op::Return, value));
            return Ok(());
        }
        if spec.is("stmt.break", w) {
            self.spill(stmts, stack);
            stmts.push(call(Op::Break, Vec::new()));
            return Ok(());
        }
        if spec.is("stmt.continue", w) {
            self.spill(stmts, stack);
            stmts.push(call(Op::Continue, Vec::new()));
            return Ok(());
        }
        if spec.is("stmt.for", w) {
            return Err(format!("'{}' needs a quoted name before it", w));
        }
        if let Some(k) = spec.list("stack.program.open").iter().position(|o| o == w) {
            let close = spec.list("stack.program.close")[k].clone();
            self.hidden += 1;
            let name = format!("<program{}>", self.hidden);
            self.scopes.push(Scope { owns: Owns::All, names: Vec::new(), params: Vec::new(), param_slots: Vec::new(), postfix: true });
            let (mut s, left) = self.postfix_block(std::slice::from_ref(&close), Run::Program)?;
            self.want_lex(&close)?;
            if let Some(v) = left {
                s.push(call(Op::Return, vec![v]));
            }
            let scope = self.scopes.pop().unwrap();
            let mut params = scope.params;
            let mut param_slots = scope.param_slots;
            params.reverse();
            param_slots.reverse();
            let program = Program { name, params, param_slots, names: scope.names, catches: Catch::Return, body: seq(s) };
            stack.push(lit(Value::Code(Rc::new(program))));
            return Ok(());
        }
        if spec.one("syntax.array.open") == Some(w) {
            let close = spec.one("syntax.array.close").unwrap().to_string();
            let (inner, items) = self.postfix_body(std::slice::from_ref(&close), Run::Block)?;
            self.want_lex(&close)?;
            if !inner.is_empty() {
                self.spill(stmts, stack);
                stmts.extend(inner);
            }
            stack.push(call(Op::Array, items));
            return Ok(());
        }
        if spec.is("stack.dup", w) {
            let a = self.pop(stack)?;
            stack.push(a);
            self.spill(stmts, stack);
            let a = stack.pop().unwrap();
            let b = copy_pure(&a);
            stack.push(a);
            stack.push(b);
            return Ok(());
        }
        if spec.is("stack.drop", w) {
            let a = self.pop(stack)?;
            if !pure(&a) {
                self.spill(stmts, stack);
                stmts.push(a);
            }
            return Ok(());
        }
        if spec.is("stack.swap", w) || spec.is("stack.over", w) || spec.is("stack.rot", w) {
            let n = if spec.is("stack.rot", w) { 3 } else { 2 };
            let mut taken = Vec::new();
            for _ in 0..n {
                taken.push(self.pop(stack)?);
            }
            taken.reverse();
            stack.extend(taken);
            self.spill(stmts, stack);
            let len = stack.len();
            if spec.is("stack.swap", w) {
                stack.swap(len - 1, len - 2);
            } else if spec.is("stack.over", w) {
                let a = copy_pure(&stack[len - 2]);
                stack.push(a);
            } else {
                let a = stack.remove(len - 3);
                stack.push(a);
            }
            return Ok(());
        }
        if spec.is("stack.eval", w) {
            let program = self.pop(stack)?;
            let arity = match &program {
                Node::Literal(Value::Code(p)) => Arity { takes: p.params.len(), leaves: leaves_value(&p.body) },
                _ if self.strict => return Err("eval needs a program written out where it is used".to_string()),
                _ => Arity { takes: 0, leaves: false },
            };
            let mut args = Vec::new();
            for _ in 0..arity.takes {
                args.push(self.pop(stack)?);
            }
            args.reverse();
            let node = run(program, args);
            if arity.leaves {
                stack.push(node);
            } else {
                self.spill(stmts, stack);
                stmts.push(node);
            }
            return Ok(());
        }
        if let Some(op) = spec.binary.get(w).copied() {
            let b = self.pop(stack)?;
            let a = self.pop(stack)?;
            stack.push(call(op.op, vec![a, b]));
            return Ok(());
        }
        if let Some(op) = spec.unary.get(w).copied() {
            let a = self.pop(stack)?;
            stack.push(call(op.op, vec![a]));
            return Ok(());
        }
        if let Some(op) = spec.builtins.get(w).copied() {
            let (takes, leaves) = match op {
                Op::Push | Op::Put => return Err(format!("'{}' needs a quoted name before it", w)),
                Op::Extern | Op::Range => return Err(format!("'{}' has no postfix form", w)),
                Op::Emit | Op::Print | Op::Write | Op::Error => (1, false),
                Op::CharAt | Op::Get | Op::Real => (2, true),
                _ => (1, true),
            };
            let mut args = Vec::new();
            for _ in 0..takes {
                args.push(self.pop(stack)?);
            }
            args.reverse();
            let node = Node::Call(Target::Op(op, Rc::from(w)), args);
            if leaves {
                stack.push(node);
            } else {
                self.spill(stmts, stack);
                stmts.push(node);
            }
            return Ok(());
        }
        if t.kind != Tk::Word {
            return Err(format!("Unexpected '{}'", w));
        }
        if let Some(arity) = self.assumed.get(w).copied() {
            let mut args = Vec::new();
            for _ in 0..arity.takes {
                args.push(self.pop(stack)?);
            }
            args.reverse();
            let target = self.load(w);
            let node = run(target, args);
            if arity.leaves {
                stack.push(node);
            } else {
                self.spill(stmts, stack);
                stmts.push(node);
            }
            return Ok(());
        }
        let node = self.load(w);
        stack.push(node);
        Ok(())
    }
}

fn copy_pure(node: &Node) -> Node {
    match node {
        Node::Literal(v) => Node::Literal(v.clone()),
        Node::Load(s) => Node::Load(s.clone()),
        Node::Call(Target::Op(op, name), args) => Node::Call(Target::Op(*op, name.clone()), args.iter().map(copy_pure).collect()),
        _ => unreachable!("only pure nodes are copied"),
    }
}

// ---------- numbers

pub fn number(text: &str, spec: &Spec) -> Out<Value> {
    if let Some(p) = spec.one("lexical.number.hex_prefix") {
        if let Some(d) = text.strip_prefix(p) {
            return BigInt::parse_bytes(d.as_bytes(), 16).map(Value::big).ok_or_else(|| format!("Invalid number: {}", text));
        }
    }
    let point = spec.ch("lexical.number.decimal_point");
    if let Some(mark) = spec.ch("lexical.number.base_marker").filter(|m| text.contains(*m)) {
        let (num, den) = based(text, mark, point, spec.ch("lexical.number.exponent_marker"))?;
        return Ok(if den == BigInt::from(1) { Value::big(num) } else { arith::build(num, den, Some(figures(text))) });
    }
    if let Some(p) = point {
        if let Some(dot) = text.find(p) {
            let (w, f) = (&text[..dot], &text[dot + p.len_utf8()..]);
            let scale = BigInt::from(10).pow(f.len() as u32);
            let w: BigInt = if w.is_empty() { BigInt::from(0) } else { w.parse().map_err(|_| format!("Invalid number: {}", text))? };
            let f: BigInt = f.parse().map_err(|_| format!("Invalid number: {}", text))?;
            return Ok(arith::build(w * &scale + f, scale, Some(figures(text))));
        }
    }
    text.parse::<BigInt>().map(Value::big).map_err(|_| format!("Invalid number: {}", text))
}

fn figures(text: &str) -> usize {
    let digits: String = text.chars().filter(char::is_ascii_alphanumeric).collect();
    let zeros = digits.chars().take_while(|c| *c == '0').count();
    digits.len().saturating_sub(zeros).max(1).max(15)
}

fn based(text: &str, mark: char, point: Option<char>, expo: Option<char>) -> Out<(BigInt, BigInt)> {
    let at = text.find(mark).unwrap();
    let base: u32 = text[..at].parse().map_err(|_| format!("Invalid base in literal '{}': base must be decimal integer", text))?;
    if !(2..=36).contains(&base) {
        return Err(format!("Invalid base {}: must be between 2 and 36", base));
    }
    let rest = &text[at + mark.len_utf8()..];
    if rest.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits after '{}'", text, mark));
    }
    let (mantissa, e) = match expo.and_then(|e| rest.find(e).map(|p| (p, e))) {
        Some((p, e)) => (&rest[..p], Some(&rest[p + e.len_utf8()..])),
        None => (rest, None),
    };
    let (whole, frac) = match point.and_then(|d| mantissa.find(d).map(|p| (p, d))) {
        Some((p, d)) => (&mantissa[..p], Some(&mantissa[p + d.len_utf8()..])),
        None => (mantissa, None),
    };
    let digits = |s: &str| -> Out<BigInt> {
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
    let mut num = digits(whole)?;
    let mut den = BigInt::from(1);
    match frac {
        Some(f) if !f.is_empty() => {
            den = BigInt::from(base).pow(f.len() as u32);
            num = num * &den + digits(f)?;
        }
        Some(_) => return Err(format!("Invalid base-N literal '{}': missing digits after '.'", text)),
        None => {}
    }
    if let Some(e) = e {
        if e.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits after exponent marker", text));
        }
        let e = digits(e)?.to_u32().ok_or_else(|| format!("Invalid base-N literal '{}': exponent too large", text))?;
        num *= BigInt::from(base).pow(e);
    }
    Ok((num, den))
}
