// Tokens to the seven forms, in one pass over the tokens. Names resolve
// to addresses here: a layer per routine, functions and bare blocks
// holding names, arms and loop bodies holding none and making no frame.
// RPLumen is read with a symbolic stack of forms.

use std::collections::HashMap;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::math;
use crate::scan::{Shape, Token};
use crate::table::{Blocks, Table};
use crate::form::{Input, Traps, Form, Prim, Routine, Address, Callee};
use crate::data::Value;

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

struct Layer {
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
    table: &'a Table,
    forks: Vec<Fork>,
    tokens: &'a [Token],
    pos: usize,
    layers: Vec<Layer>,
    gensyms: usize,
    pub presumed: HashMap<String, Signature>,
    pub seen: HashMap<String, Signature>,
    strict: bool,
    /// Settings of hidden globals for `static` names, run where the function is defined.
    statics: Vec<Form>,
}

pub struct Built {
    pub program: Rc<Routine>,
    pub globals: Vec<String>,
    pub seen: HashMap<String, Signature>,
}

/// What a run of postfix words is part of, which says where it stops.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Body,
    Quoted,
    Single,
}

pub fn build(tokens: &[Token], table: &Table, seeded: &[String], assumed: HashMap<String, Signature>, strict: bool) -> Res<Built> {
    let top = Layer { holds: Holds::Every, idents: seeded.to_vec(), formals: Vec::new(), formal_slots: Vec::new(), rpn: false, aliases: Vec::new() };
    let mut r = Builder { table, forks: Vec::new(), tokens, pos: 0, layers: vec![top], gensyms: 0, presumed: assumed, seen: HashMap::new(), strict, statics: Vec::new() };
    let body = if table.rpn {
        let (mut stmts, rest) = r.rpn_body(&[], Mode::Body)?;
        if !r.exhausted() {
            return Err(format!("Unexpected '{}'", r.look().lexeme));
        }
        stmts.extend(rest.into_iter().filter(|n| !inert(n)));
        sequence(stmts)
    } else {
        // A top-level function may be bound ahead of everything else, so a
        // call written above it finds it (ext.stmt.function.hoisted).
        let mut stmts = Vec::new();
        let mut ahead = Vec::new();
        r.skip_line_ends();
        while !r.exhausted() {
            let defines = table.flag("ext.stmt.function.hoisted") && r.key("stmt.function");
            let stmt = r.stmt()?;
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
    let top = r.layers.pop().unwrap();
    let program = Routine { ident: "<program>".into(), formals: Vec::new(), formal_slots: Vec::new(), idents: top.idents.clone(), frameless: false, traps: Traps::Naught, body };
    Ok(Built { program: Rc::new(program), globals: top.idents, seen: r.seen })
}

// ---------- node builders


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

fn inert(node: &Form) -> bool {
    match node {
        Form::Const(_) | Form::Read(_) => true,
        Form::Dyad { a, b, .. } => [a, b].iter().all(|o| match o {
            Input::Form(n) => inert(n),
            _ => true,
        }),
        Form::Apply(Callee::Prim(op, _), args) => {
            !matches!(op, Prim::Echo | Prim::Say | Prim::Out | Prim::Tell | Prim::Dump | Prim::Define | Prim::Raise | Prim::External | Prim::Append | Prim::Replace | Prim::Yield | Prim::Leave | Prim::Resume | Prim::Choose | Prim::Both | Prim::Either | Prim::Seq)
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
        t.shape == Shape::LineEnd || (t.shape == Shape::Sign && self.table.spells("stmt.terminator", &t.lexeme))
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
        Some(self.global_address(&target))
    }

    fn address_to_read(&mut self, name: &str) -> Address {
        if let Some(slot) = self.aliased(name) {
            return slot;
        }
        let frameless = |holds: Holds| holds == Holds::Nothing;
        let mut depth = 0;
        let mut found: Option<(usize, usize, usize)> = None;
        for (i, scope) in self.layers.iter().enumerate().rev() {
            if let Some(index) = scope.idents.iter().rposition(|n| n == name).filter(|_| scope.holds != Holds::Nothing) {
                found = Some((i, depth, index));
                break;
            }
            if scope.holds == Holds::Every && i != 0 {
                break;
            }
            if !frameless(scope.holds) {
                depth += 1;
            }
        }
        let global = self.global_at(name);
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
        if let Some(slot) = self.aliased(name) {
            return slot;
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

    fn read(&mut self, name: &str) -> Form {
        Form::Read(self.address_to_read(name))
    }

    fn write(&mut self, name: &str, value: Form) -> Form {
        let slot = self.address_to_write(name);
        // `x = x + k` steps the binding in place.
        if let Form::Dyad { op: Prim::Plus, a, b, .. } = &value {
            if let (Input::Address(read), Input::Const(Value::Small(k))) = (a, b) {
                if read.ident == slot.ident && read.up == slot.up && read.at == slot.at && read.fallback == slot.fallback {
                    return Form::Bump { slot, by: *k };
                }
            }
        }
        Form::Write(slot, Box::new(value))
    }

    /// A program value: its body reduced in a scope of its own.
    fn routine(&mut self, name: &str, holds: Holds, catches: Traps, params: Vec<String>, body: impl FnOnce(&mut Self) -> Res<Form>) -> Res<Form> {
        let param_slots = (0..params.len()).collect();
        self.layers.push(Layer { holds, idents: params.clone(), formals: Vec::new(), formal_slots: param_slots, rpn: false, aliases: Vec::new() });
        let body = body(self)?;
        let scope = self.layers.pop().unwrap();
        Ok(constant(Value::Routine(Rc::new(Routine { ident: name.to_string(), formals: params, formal_slots: scope.formal_slots, idents: scope.idents, frameless: holds == Holds::Nothing, traps: catches, body }))))
    }

    /// A branch arm or a loop body: a program that holds no names.
    fn limb(&mut self, catches: Traps, body: impl FnOnce(&mut Self) -> Res<Form>) -> Res<Form> {
        // An arm that traps nothing is read in place, as part of
        // the program around it, not as a program of its own.
        if catches == Traps::Naught {
            return body(self);
        }
        self.routine("<arm>", Holds::Nothing, catches, Vec::new(), body)
    }

    /// A branch: `if(test, «then», «otherwise»)`, the arms program values
    /// that own no names, so the chosen one runs in the frame around it.
    fn choose(&mut self, test: Form, then: Form, otherwise: Form) -> Form {
        let wrap = |name: &str, body: Form| {
            let program = Routine { ident: name.to_string(), formals: Vec::new(), formal_slots: Vec::new(), idents: Vec::new(), frameless: true, traps: Traps::Naught, body };
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
        Ok(Form::Cycle { test: Box::new(test), body: Box::new(body), step, after: false })
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
        Ok(Form::Cycle { test: Box::new(test), body: Box::new(body), step: None, after: true })
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
        self.skip_lead_word();
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
        if self.look().shape == Shape::Bare {
            if self.key("stmt.let") {
                return self.bind();
            }
            if self.key("stmt.if") {
                return self.if_stmt();
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
            if self.key("stmt.return") {
                self.advance();
                let value = if self.on_stmt_end() || self.exhausted() || self.on_any("block.close") {
                    Vec::new()
                } else {
                    vec![self.expr(0)?]
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
                let name = self.need_word("after the function keyword")?;
                return self.func(name);
            }
            if self.key("stmt.pass") {
                self.advance();
                return Ok(constant(Value::Nil));
            }
            if self.key("ext.stmt.for.c") {
                return self.three_part_for();
            }
            if self.key("ext.stmt.switch") {
                return self.switch_stmt();
            }
            if self.key("ext.stmt.global") {
                return self.global_names();
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
            let program = self.routine("<block>", Holds::Fresh, Traps::Naught, Vec::new(), |r| r.body())?;
            return Ok(invoke(program, Vec::new()));
        }
        self.plain_stmt()
    }

    /// The names after `global`, or `static` with their settings, up to
    /// the end of the statement, separated as call arguments are.
    fn listed_names(&mut self, what: &str, mut each: impl FnMut(&mut Self, String) -> Res<()>) -> Res<Form> {
        self.advance();
        let sep = self.table.single("syntax.call.separator").map(str::to_string);
        loop {
            let name = self.need_word(what)?;
            each(self, name)?;
            match &sep {
                Some(s) if self.sign(s) => self.advance(),
                _ => return Ok(constant(Value::Nil)),
            };
        }
    }

    /// `global a, b;`: the names mean the globals inside this function.
    fn global_names(&mut self) -> Res<Form> {
        self.listed_names("after the global keyword", |r, name| {
            let owner = r.layers.iter_mut().rev().find(|s| s.holds == Holds::Every).expect("the top layer");
            owner.aliases.push((name.clone(), name));
            Ok(())
        })
    }

    /// `static x = e;`: x means a hidden global, set where the function
    /// is defined, so it keeps its value between calls. The setting is
    /// read outside the function's layer, where it will run.
    fn static_names(&mut self) -> Res<Form> {
        if self.layers.len() < 2 {
            return Err("static belongs inside a function".to_string());
        }
        self.listed_names("after the static keyword", |r, name| {
            r.gensyms += 1;
            let hidden = format!("#static{}", r.gensyms);
            let inner = r.layers.pop().expect("the function's layer");
            let value = if r.on_assign() {
                r.advance();
                r.expr(0)
            } else {
                Ok(constant(Value::Nil))
            };
            let slot = r.global_address(&hidden);
            r.layers.push(inner);
            let value = value?;
            r.statics.push(Form::Write(slot, Box::new(value)));
            let owner = r.layers.iter_mut().rev().find(|s| s.holds == Holds::Every).expect("the function's layer");
            owner.aliases.push((name, hidden));
            Ok(())
        })
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
        let looped = Form::Cycle { test: Box::new(test), body: Box::new(body), step: Some(Box::new(step)), after: false };
        Ok(sequence(vec![init, looped]))
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
        let k = opens.iter().position(|o| self.lexeme_of(o)).ok_or_else(|| format!("Expected '{}' to open the switch, got '{}'", opens[0], self.look().lexeme))?;
        self.advance();
        let close = table.strings("block.close")[k].clone();
        let mut stops = vec![close.clone()];
        stops.extend(table.strings("ext.stmt.case").iter().cloned());
        stops.extend(table.strings("ext.stmt.default").iter().cloned());
        // (test for the section, or None for the default; its body)
        let mut sections: Vec<(Option<Form>, Form)> = Vec::new();
        self.skip_line_ends();
        while !self.lexeme_of(&close) && !self.exhausted() {
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
            self.advance();
            let body = self.limb(Traps::Naught, |r| r.stmts_until(&stops))?;
            sections.push((test, body));
        }
        self.need_lexeme(&close)?;
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
        let pass = Form::Cycle { test: Box::new(constant(Value::Flag(true))), body: Box::new(sequence(run)), step: None, after: true };
        Ok(sequence(vec![keep, Form::Write(start, Box::new(choice)), pass]))
    }

    /// A statement without a keyword: a step, a bare call, an
    /// assignment or an expression.
    fn plain_stmt(&mut self) -> Res<Form> {
        // `x++;` / `++x;`: as a statement only the stepping counts.
        let (here, next) = (self.look().clone(), self.glance(1).clone());
        if let (Some(by), Shape::Bare) = (self.step_by(&here), next.shape) {
            self.pos += 2;
            return Ok(self.stepped(&next.lexeme, by));
        }
        if let (Shape::Bare, Some(by)) = (here.shape, self.step_by(&next)) {
            let reserved = self.table.keywords.contains(&here.lexeme) || self.table.prims.contains_key(&here.lexeme);
            if !reserved {
                self.pos += 2;
                return Ok(self.stepped(&here.lexeme, by));
            }
        }
        if self.table.flag("ext.syntax.call.bare") && here.shape == Shape::Bare {
            // A builtin with no bracket after it; echo always, since a
            // bracket after echo opens a group, not its arguments.
            let op = self.table.prims.get(&here.lexeme).copied();
            let bracketed = self.table.single("syntax.call.open").map_or(false, |o| next.shape == Shape::Sign && next.lexeme == o);
            let bare = match op {
                Some(Prim::Tell) => true,
                Some(Prim::Append | Prim::Replace | Prim::Define) | None => false,
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

    /// `x = x + by`, which the writer folds into a Bump.
    fn stepped(&mut self, name: &str, by: i64) -> Form {
        let sum = prim_call(Prim::Plus, vec![self.read(name), constant(Value::Small(by))]);
        self.write(name, sum)
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
        while self.glance(ahead).shape == Shape::LineEnd || (self.glance(ahead).shape == Shape::Sign && self.table.spells("stmt.terminator", &self.glance(ahead).lexeme)) {
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

    fn for_stmt(&mut self) -> Res<Form> {
        let table = self.table;
        self.advance();
        let var = self.need_word("as the loop variable")?;
        if !self.key("stmt.for.in") {
            return Err(format!("Expected '{}' after for loop variable, got: {}", table.single("stmt.for.in").unwrap_or("in"), self.look().lexeme));
        }
        self.advance();
        let ranged = self.look().shape == Shape::Bare
            && table.prims.get(&self.look().lexeme) == Some(&Prim::Span)
            && table.single("syntax.call.open").map_or(false, |o| self.glance(1).shape == Shape::Sign && self.glance(1).lexeme == o);
        let (start, end) = if ranged {
            self.advance();
            self.advance();
            let start = self.expr(0)?;
            if let Some(sep) = table.single("syntax.call.separator") {
                self.need_sign(sep, "between the range bounds")?;
            }
            let end = self.expr(0)?;
            self.need_sign(table.single("syntax.call.close").unwrap(), "after the range")?;
            (start, end)
        } else {
            let tier = table.strings("op.range").iter().filter_map(|r| table.precedence.get(r.as_str())).min().copied().unwrap_or(0);
            let start = self.expr(tier + 1)?;
            if !(self.look().shape == Shape::Sign && table.spells("op.range", &self.look().lexeme)) {
                return Err("A for loop needs a range: start..end".to_string());
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
                    return self.func(name);
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

    fn func(&mut self, name: String) -> Res<Form> {
        let table = self.table;
        let open = table.single("syntax.call.open").ok_or_else(|| "This language has no call syntax".to_string())?;
        let close = table.single("syntax.call.close").unwrap();
        self.need_sign(open, "after function name")?;
        let typed = table.flag("stmt.let.type_first");
        let mut params = Vec::new();
        while !self.sign(close) && !self.exhausted() {
            if typed {
                let t = self.need_word("as a parameter type")?;
                if !table.spells("stmt.let", &t) {
                    return Err(format!("'{}' is not a type word", t));
                }
                if self.look().shape == Shape::Bare {
                    params.push(self.advance().lexeme);
                }
            } else {
                params.push(self.need_word("as a parameter name")?);
                if self.look().shape == Shape::Sign && table.spells("stmt.let.annotation", &self.look().lexeme) {
                    self.advance();
                    self.need_word("as a type name")?;
                }
            }
            if let Some(sep) = table.single("syntax.call.separator") {
                if self.sign(sep) {
                    self.advance();
                }
            }
            if self.look().shape == Shape::Sign && table.spells("stmt.terminator", &self.look().lexeme) {
                self.advance();
            }
        }
        self.need_sign(close, "after parameters")?;
        if self.look().shape == Shape::Sign && table.spells("stmt.function.returns", &self.look().lexeme) {
            self.advance();
            self.need_word("as a return type")?;
        }
        let declared = self.look().shape == Shape::Sign && table.spells("stmt.terminator", &self.look().lexeme);
        let statics_before = self.statics.len();
        let program = self.routine(&name, Holds::Every, Traps::Yields, params, |r| {
            let mut items = Vec::new();
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
        let mut items: Vec<Form> = self.statics.drain(statics_before..).collect();
        items.push(self.write(&name, program));
        Ok(sequence(items))
    }

    fn write_or_expr(&mut self) -> Res<Form> {
        let expr = self.expr(0)?;
        let compound = if self.look().shape == Shape::Sign { self.table.compound.get(&self.look().lexeme).copied() } else { None };
        if !self.on_assign() && compound.is_none() {
            return Ok(expr);
        }
        let assign = self.advance();
        let value = self.expr(0)?;
        match expr {
            // x op= e is x = x op e.
            Form::Read(slot) => match compound {
                Some(op) => {
                    let current = self.read(&slot.ident);
                    let combined = prim_call(op, vec![current, value]);
                    Ok(self.write(&slot.ident, combined))
                }
                None => Ok(self.write(&slot.ident, value)),
            },
            _ if compound.is_some() => Err(format!("'{}' needs a plain variable on its left", assign.lexeme)),
            Form::Apply(Callee::Prim(Prim::At, _), mut args) if args.len() == 2 => {
                let index = args.pop().unwrap();
                match args.pop().unwrap() {
                    Form::Read(slot) => {
                        let target = self.read(&slot.ident);
                        Ok(prim_call(Prim::Replace, vec![target, index, value]))
                    }
                    _ => Err("Invalid assignment target".to_string()),
                }
            }
            _ => Err(format!("Invalid assignment target before '{}'", assign.lexeme)),
        }
    }

    // ---------- expressions

    fn expr(&mut self, floor: u32) -> Res<Form> {
        let table = self.table;
        let mut left = self.monadic_expr()?;
        loop {
            let t = self.look();
            if t.shape != Shape::Sign && t.shape != Shape::Bare {
                break;
            }
            let text = t.lexeme.clone();
            if table.spells("op.pipe", &text) {
                if table.precedence.get(&text).copied().unwrap_or(0) < floor {
                    break;
                }
                self.advance();
                let name = self.need_word("after the pipe")?;
                let mut args = vec![left];
                if let Some(open) = table.single("syntax.call.open") {
                    if self.sign(open) {
                        self.advance();
                        args.extend(self.args("syntax.call.close", "syntax.call.separator")?);
                    }
                }
                left = self.named_call(&name, args)?;
                continue;
            }
            let Some(op) = table.dyadic.get(&text).copied() else {
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
            self.advance();
            let floor_right = if op.right_assoc { op.level } else { op.level + 1 };
            left = match op.prim {
                // The right side is a program, run only when the left leaves it open.
                Prim::Both | Prim::Either => {
                    let lazy = self.limb(Traps::Naught, |r| r.expr(floor_right))?;
                    prim_call(op.prim, vec![left, lazy])
                }
                other => {
                    let right = self.expr(floor_right)?;
                    prim_call(other, vec![left, right])
                }
            };
        }
        Ok(left)
    }

    fn monadic_expr(&mut self) -> Res<Form> {
        let table = self.table;
        let t = self.look().clone();
        if let (Some(by), Shape::Bare) = (self.step_by(&t), self.glance(1).shape) {
            // ++x is the stepped value.
            self.advance();
            let name = self.advance().lexeme;
            let step = self.stepped(&name, by);
            return Ok(sequence(vec![step, self.read(&name)]));
        }
        if matches!(t.shape, Shape::Sign | Shape::Bare) {
            if let Some(op) = table.monadic.get(&t.lexeme).copied() {
                self.advance();
                let operand = self.expr(op.level)?;
                return Ok(prim_call(op.prim, vec![operand]));
            }
            if t.shape == Shape::Sign && table.spells("ext.op.plus", &t.lexeme) {
                // A plus sign leaves its operand as it is, bound like a negation.
                self.advance();
                let tier = table.monadic.values().map(|m| m.level).max().unwrap_or(0);
                return self.expr(tier);
            }
        }
        let node = match t.shape {
            Shape::Numeral => {
                self.advance();
                constant(numeral(&t.lexeme, table)?)
            }
            Shape::Quote => {
                self.advance();
                constant(Value::text(&t.lexeme))
            }
            Shape::Bare => {
                self.advance();
                if table.spells("literal.true", &t.lexeme) {
                    constant(Value::Flag(true))
                } else if table.spells("literal.false", &t.lexeme) {
                    constant(Value::Flag(false))
                } else if table.spells("literal.null", &t.lexeme) {
                    constant(Value::Nil)
                } else if table.single("syntax.call.open").map_or(false, |o| self.sign(o)) {
                    self.advance();
                    let mut args = self.args("syntax.call.close", "syntax.call.separator")?;
                    if table.prims.get(&t.lexeme) == Some(&Prim::Define) {
                        // define("NAME", v) binds the global NAME here; its value is true.
                        let (Some(Form::Const(Value::Text(name))), 2) = (args.first(), args.len()) else {
                            return Err(format!("{}() needs a quoted name and a value", t.lexeme));
                        };
                        let slot = self.global_address(&name.to_string());
                        let value = args.pop().unwrap();
                        sequence(vec![Form::Write(slot, Box::new(value)), constant(Value::Flag(true))])
                    } else {
                        self.named_call(&t.lexeme, args)?
                    }
                } else if let Some(by) = self.step_by(&self.look().clone()) {
                    // x++ is the value before the step, kept aside.
                    self.advance();
                    self.gensyms += 1;
                    let aside = format!("#was{}", self.gensyms);
                    let before = self.read(&t.lexeme);
                    let keep = self.write(&aside, before);
                    let step = self.stepped(&t.lexeme, by);
                    sequence(vec![keep, step, self.read(&aside)])
                } else {
                    self.read(&t.lexeme)
                }
            }
            Shape::Sign => {
                if table.single("syntax.group.open") == Some(t.lexeme.as_str()) {
                    self.advance();
                    let inner = self.expr(0)?;
                    self.need_sign(table.single("syntax.group.close").unwrap(), "to close a group")?;
                    inner
                } else if table.single("syntax.array.open") == Some(t.lexeme.as_str()) {
                    self.advance();
                    let items = self.args("syntax.array.close", "syntax.array.separator")?;
                    prim_call(Prim::MakeArray, items)
                } else {
                    return Err(format!("Unexpected token: {}", t.lexeme));
                }
            }
            _ => return Err("Expected an expression".to_string()),
        };
        self.subscript(node)
    }

    fn subscript(&mut self, mut node: Form) -> Res<Form> {
        let (Some(open), Some(close)) = (self.table.single("op.index.open"), self.table.single("op.index.close")) else { return Ok(node) };
        while self.sign(open) {
            self.advance();
            let index = self.expr(0)?;
            self.need_sign(close, "after array index")?;
            node = prim_call(Prim::At, vec![node, index]);
        }
        Ok(node)
    }

    fn args(&mut self, close_key: &str, sep_key: &str) -> Res<Vec<Form>> {
        let close = self.table.single(close_key).unwrap().to_string();
        let sep = self.table.single(sep_key).map(str::to_string);
        let mut items = Vec::new();
        while !self.sign(&close) {
            if self.exhausted() {
                return Err(format!("Expected '{}'", close));
            }
            if self.look().shape == Shape::Bare && self.glance(1).shape == Shape::Sign && self.table.spells("syntax.call.label", &self.glance(1).lexeme) {
                self.pos += 2;
            }
            items.push(self.expr(0)?);
            if let Some(s) = &sep {
                if self.sign(s) {
                    self.advance();
                }
            }
        }
        self.advance();
        Ok(items)
    }

    fn named_call(&mut self, name: &str, args: Vec<Form>) -> Res<Form> {
        match self.table.prims.get(name).copied() {
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
                while self.look().shape == Shape::Sign && self.table.spells("stmt.terminator", &self.look().lexeme) {
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
            let separator = t.shape == Shape::LineEnd || (t.shape == Shape::Sign && self.table.spells("stmt.terminator", &t.lexeme));
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
            self.layers.push(Layer { holds: Holds::Every, idents: Vec::new(), formals: Vec::new(), formal_slots: Vec::new(), rpn: true, aliases: Vec::new() });
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
            let program = Routine { ident: name, formals: params, formal_slots: param_slots, idents: scope.idents, frameless: false, traps: Traps::Yields, body: sequence(s) };
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
                Prim::Define => return Err(format!("'{}' has no postfix form", w)),
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

pub fn numeral(text: &str, table: &Table) -> Res<Value> {
    if let Some(p) = table.single("lexical.number.hex_prefix") {
        if let Some(d) = text.strip_prefix(p) {
            return BigInt::parse_bytes(d.as_bytes(), 16).map(Value::from_big).ok_or_else(|| format!("Invalid number: {}", text));
        }
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
        let power: i32 = power.parse().map_err(|_| format!("Invalid number: {}", text))?;
        let (above, beneath) = match numeral(before, table)? {
            Value::Frac(e) => (e.above.clone(), e.beneath.clone()),
            Value::Small(n) => (BigInt::from(n), BigInt::from(1)),
            Value::Huge(n) => ((*n).clone(), BigInt::from(1)),
            _ => return Err(format!("Invalid number: {}", text)),
        };
        let ten = BigInt::from(10).pow(power.unsigned_abs());
        let (above, beneath) = if power < 0 { (above, beneath * ten) } else { (above * ten, beneath) };
        return Ok(math::make_number(above, beneath, Some(digit_run(before))));
    }
    if let Some(p) = point {
        if let Some(dot) = text.find(p) {
            let (w, f) = (&text[..dot], &text[dot + p.len_utf8()..]);
            let scale = BigInt::from(10).pow(f.len() as u32);
            let w: BigInt = if w.is_empty() { BigInt::from(0) } else { w.parse().map_err(|_| format!("Invalid number: {}", text))? };
            let f: BigInt = f.parse().map_err(|_| format!("Invalid number: {}", text))?;
            return Ok(math::make_number(w * &scale + f, scale, Some(digit_run(text))));
        }
    }
    text.parse::<BigInt>().map(Value::from_big).map_err(|_| format!("Invalid number: {}", text))
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
