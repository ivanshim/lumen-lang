// Stage 3: Reduce — tokens → instruction tree.
//
// Statements are recognised by the keywords the schema assigns to each
// statement form; expressions are parsed by precedence climbing over the
// schema's operator tables. Every construct reduces to the primitive
// instruction set: `for` and `until` loops, function definitions, indexed
// assignment and the pipe operator are desugared here.

use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use super::_1_ingest::{Kind, Token};
use super::instruction::{Instruction, Target, TransferKind};
use super::value::{Function, Value};
use crate::schema::{spelled, Assoc, BlockStyle, Builtin, LanguageSchema, Op};

pub fn parse(tokens: &[Token], schema: &LanguageSchema) -> Result<Instruction, String> {
    let mut parser = Parser { tokens, pos: 0, schema, hidden: 0 };
    parser.program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    schema: &'a LanguageSchema,
    /// Counter for hidden bindings introduced by desugaring.
    hidden: usize,
}

const EPSILON: f32 = 0.001;

impl<'a> Parser<'a> {
    // ---------- token access ----------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_at(&self, offset: usize) -> &Token {
        &self.tokens[(self.pos + offset).min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn at_end(&self) -> bool {
        self.peek().kind == Kind::Eof
    }

    fn is_op(&self, op: &str) -> bool {
        self.peek().is(Kind::Op, op)
    }

    fn is_terminator(&self) -> bool {
        let tok = self.peek();
        tok.kind == Kind::Newline || (tok.kind == Kind::Op && self.schema.is_terminator(&tok.text))
    }

    /// Whether the current token is `text`, as an operator or a word: block
    /// delimiters may be either (`{` or `begin`).
    fn at_lexeme(&self, text: &str) -> bool {
        let tok = self.peek();
        (tok.kind == Kind::Op || tok.kind == Kind::Word) && tok.text == text
    }

    /// The position in `list` of the current token, if it is one of them.
    fn at_one_of(&self, list: &[String]) -> Option<usize> {
        list.iter().position(|lex| self.at_lexeme(lex))
    }

    fn skip_terminators(&mut self) {
        while !self.at_end() && self.is_terminator() {
            self.advance();
        }
    }

    fn expect_op(&mut self, op: &str, context: &str) -> Result<(), String> {
        if self.is_op(op) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", op, context, self.peek().text))
        }
    }

    fn expect_word(&mut self, context: &str) -> Result<String, String> {
        if self.peek().kind == Kind::Word {
            Ok(self.advance().text)
        } else {
            Err(format!("Expected identifier {}, got '{}'", context, self.peek().text))
        }
    }

    fn hidden_name(&mut self, purpose: &str) -> String {
        self.hidden += 1;
        format!("#{}{}", purpose, self.hidden)
    }

    // ---------- statements ----------

    fn program(&mut self) -> Result<Instruction, String> {
        if self.schema.structure.postfix {
            return self.postfix_program();
        }
        let mut stmts = Vec::new();
        self.skip_terminators();
        while !self.at_end() {
            stmts.push(self.statement()?);
            self.skip_terminators();
        }
        Ok(Instruction::Sequence(stmts))
    }

    /// Statements up to a token in `stops` (not consumed) or the end.
    fn body_until(&mut self, stops: &[String]) -> Result<Instruction, String> {
        let mut stmts = Vec::new();
        self.skip_terminators();
        while self.at_one_of(stops).is_none() && !self.at_end() {
            stmts.push(self.statement()?);
            self.skip_terminators();
        }
        Ok(Instruction::Sequence(stmts))
    }

    /// Drop a block-intro token (Python's `:`, Lua's `then`) if present.
    fn skip_block_intro(&mut self) {
        let structure: &'a crate::schema::Structure = &self.schema.structure;
        if self.at_one_of(&structure.block_intro).is_some() {
            self.advance();
        }
    }

    /// A block after a statement header. For indentation and brace styles
    /// the opener at position i pairs with the closer at position i; for the
    /// keyword style there is no opener and any closer ends the body.
    fn block(&mut self) -> Result<Instruction, String> {
        let structure: &'a crate::schema::Structure = &self.schema.structure;
        self.skip_block_intro();
        self.skip_terminators();
        match structure.blocks {
            BlockStyle::Indentation | BlockStyle::Braces => {
                let i = self
                    .at_one_of(&structure.block_open)
                    .ok_or_else(|| format!("Expected '{}' to open a block, got '{}'", structure.block_open[0], self.peek().text))?;
                self.advance();
                let close = std::slice::from_ref(&structure.block_close[i]);
                let body = self.body_until(close)?;
                self.expect_close(&close[0])?;
                Ok(body)
            }
            BlockStyle::Keyword => {
                let body = self.body_until(&structure.block_close)?;
                self.expect_any_close()?;
                Ok(body)
            }
        }
    }

    fn expect_close(&mut self, close: &str) -> Result<(), String> {
        if self.at_lexeme(close) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", close, self.peek().text))
        }
    }

    fn expect_any_close(&mut self) -> Result<(), String> {
        let structure: &'a crate::schema::Structure = &self.schema.structure;
        if self.at_one_of(&structure.block_close).is_some() {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}' to close a block, got '{}'", structure.block_close[0], self.peek().text))
        }
    }

    fn statement(&mut self) -> Result<Instruction, String> {
        let schema: &'a LanguageSchema = self.schema;
        let s = &schema.statements;
        if self.peek().kind == Kind::Word {
            let word = self.peek().text.clone();
            if s.binding.as_ref().map_or(false, |b| spelled(&b.keyword, &word)) {
                return self.binding();
            }
            if s.branch.as_ref().map_or(false, |b| spelled(&b.keyword, &word)) {
                return self.branch();
            }
            if spelled(&s.loop_while, &word) {
                return self.while_loop();
            }
            if spelled(&s.loop_until, &word) {
                return self.until_loop();
            }
            if s.loop_for.as_ref().map_or(false, |f| spelled(&f.keyword, &word)) {
                return self.for_loop();
            }
            if spelled(&s.return_, &word) {
                return self.return_stmt();
            }
            if spelled(&s.break_, &word) {
                self.advance();
                return Ok(Instruction::transfer(TransferKind::Break, None));
            }
            if spelled(&s.continue_, &word) {
                self.advance();
                return Ok(Instruction::transfer(TransferKind::Continue, None));
            }
            if spelled(&s.function, &word) {
                return self.function_def();
            }
            if spelled(&s.pass, &word) {
                self.advance();
                return Ok(Instruction::Sequence(Vec::new()));
            }
        }
        if schema.structure.blocks == BlockStyle::Braces && self.at_one_of(&schema.structure.block_open).is_some() {
            return Ok(Instruction::Scope(Box::new(self.block()?)));
        }
        self.assignment_or_expression()
    }

    fn binding(&mut self) -> Result<Instruction, String> {
        let schema: &'a LanguageSchema = self.schema;
        let binding = schema.statements.binding.as_ref().expect("binding form present");
        if binding.type_first {
            return self.typed_declaration();
        }
        self.advance(); // keyword
        if self.peek().kind == Kind::Word && spelled(&binding.mutable_modifier, &self.peek().text) {
            self.advance();
        }
        let name = self.expect_word("after the binding keyword")?;
        if self.peek().kind == Kind::Op && spelled(&binding.type_annotation, &self.peek().text) {
            self.advance();
            self.expect_word("as a type name")?;
        }
        if self.is_terminator() || self.at_end() {
            // A declaration without a value (`var x: integer;`, `let x;`) binds null.
            return Ok(Instruction::assign(name, Instruction::Literal(Value::Null)));
        }
        self.expect_assignment("in a binding")?;
        let value = self.expression(0.0)?;
        Ok(Instruction::assign(name, value))
    }

    /// With `stmt.let.type_first` the keyword is the type (C's `int`): the
    /// call bracket after the name makes a function definition, the end of
    /// the statement a null binding, and otherwise a value is assigned.
    fn typed_declaration(&mut self) -> Result<Instruction, String> {
        self.advance(); // the type
        let name = self.expect_word("after the type")?;
        if let Some(call) = self.schema.structure.call.clone() {
            if self.is_op(&call.open) {
                return self.function_rest(name);
            }
        }
        if self.is_terminator() || self.at_end() {
            return Ok(Instruction::assign(name, Instruction::Literal(Value::Null)));
        }
        self.expect_assignment("in a declaration")?;
        let value = self.expression(0.0)?;
        Ok(Instruction::assign(name, value))
    }

    fn at_assignment(&self) -> bool {
        self.peek().kind == Kind::Op && spelled(&self.schema.statements.assignment, &self.peek().text)
    }

    fn expect_assignment(&mut self, context: &str) -> Result<(), String> {
        if self.schema.statements.assignment.is_empty() {
            return Err("This language has no assignment operator".to_string());
        }
        if self.at_assignment() {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}' {}, got '{}'", self.schema.statements.assignment[0], context, self.peek().text))
        }
    }

    fn branch(&mut self) -> Result<Instruction, String> {
        let schema: &'a LanguageSchema = self.schema;
        let branch = schema.statements.branch.as_ref().expect("branch form present");
        let keyword_blocks = schema.structure.blocks == BlockStyle::Keyword;
        self.advance(); // if / elif
        let condition = self.expression(0.0)?;

        // In the keyword style the whole chain shares one closer, so each
        // body runs to an elif, an else or the closer, and only the end of
        // the chain consumes the closer.
        let then_branch = if keyword_blocks {
            self.skip_block_intro();
            let mut stops = schema.structure.block_close.clone();
            stops.extend(branch.elif_keyword.iter().cloned());
            stops.extend(branch.else_keyword.iter().cloned());
            self.body_until(&stops)?
        } else {
            self.block()?
        };

        // An else or elif may follow on the same line or after line ends.
        let mut look = 0;
        while self.peek_at(look).kind == Kind::Newline
            || (self.peek_at(look).kind == Kind::Op && self.schema.is_terminator(&self.peek_at(look).text))
        {
            look += 1;
        }
        let next = self.peek_at(look);
        let is_elif = next.kind == Kind::Word && spelled(&branch.elif_keyword, &next.text);
        let is_else = next.kind == Kind::Word && spelled(&branch.else_keyword, &next.text);

        let else_branch = if is_elif {
            self.pos += look;
            Some(self.branch()?)
        } else if is_else {
            self.pos += look;
            self.advance(); // else
            if self.peek().kind == Kind::Word && spelled(&branch.keyword, &self.peek().text) {
                Some(self.branch()?) // else if
            } else if keyword_blocks {
                let body = self.body_until(&schema.structure.block_close)?;
                self.expect_any_close()?;
                Some(body)
            } else {
                Some(self.block()?)
            }
        } else {
            if keyword_blocks {
                self.expect_any_close()?;
            }
            None
        };

        Ok(Instruction::Branch {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        })
    }

    fn while_loop(&mut self) -> Result<Instruction, String> {
        self.advance();
        let condition = self.expression(0.0)?;
        let body = self.block()?;
        Ok(Instruction::Loop { condition: Box::new(condition), body: Box::new(body), step: None })
    }

    /// `until cond { body }` runs the body first and stops once cond holds:
    /// an unconditional loop whose step breaks when the condition is true.
    fn until_loop(&mut self) -> Result<Instruction, String> {
        self.advance();
        let condition = self.expression(0.0)?;
        let body = self.block()?;
        let stop = Instruction::Branch {
            condition: Box::new(condition),
            then_branch: Box::new(Instruction::transfer(TransferKind::Break, None)),
            else_branch: None,
        };
        Ok(Instruction::Loop {
            condition: Box::new(Instruction::Literal(Value::Bool(true))),
            body: Box::new(body),
            step: Some(Box::new(stop)),
        })
    }

    /// `for v in range { body }` binds v to the range start and loops while
    /// v is below the range end, stepping by one after each pass.
    /// A for loop over a range is a counted loop: the range is loop syntax
    /// (`start..end`, or the range call), not a value.
    fn for_loop(&mut self) -> Result<Instruction, String> {
        let schema: &'a LanguageSchema = self.schema;
        let form = schema.statements.loop_for.as_ref().expect("for form present");
        self.advance();
        let var = self.expect_word("as the loop variable")?;
        if !(self.peek().kind == Kind::Word && spelled(&form.in_keyword, &self.peek().text)) {
            return Err(format!("Expected '{}' after for loop variable, got: {}", form.in_keyword[0], self.peek().text));
        }
        self.advance();
        let iterable = self.expression(0.0)?;
        let body = self.block()?;

        let (start, end) = match iterable {
            Instruction::Operate { op: Op::Range, mut operands } if operands.len() == 2 => {
                let end = operands.pop().expect("two operands");
                (operands.pop().expect("two operands"), end)
            }
            Instruction::Invoke { function, mut args }
                if schema.functions.get(&function) == Some(&Builtin::Range) && args.len() == 2 =>
            {
                let end = args.pop().expect("two arguments");
                (args.pop().expect("two arguments"), end)
            }
            _ => return Err("A for loop needs a range: start..end".to_string()),
        };

        let limit = self.hidden_name("end");
        let bind_limit = Instruction::assign(limit.clone(), end);
        let bind_var = Instruction::assign(var.clone(), start);
        let condition = Instruction::binary(Op::Lt, Instruction::Variable(var.clone()), Instruction::Variable(limit));
        let step = Instruction::assign(
            var.clone(),
            Instruction::binary(Op::Add, Instruction::Variable(var), Instruction::Literal(Value::Number(BigInt::from(1)))),
        );
        Ok(Instruction::Sequence(vec![
            bind_limit,
            bind_var,
            Instruction::Loop { condition: Box::new(condition), body: Box::new(body), step: Some(Box::new(step)) },
        ]))
    }

    fn return_stmt(&mut self) -> Result<Instruction, String> {
        let structure: &'a crate::schema::Structure = &self.schema.structure;
        self.advance();
        if self.is_terminator() || self.at_end() || self.at_one_of(&structure.block_close).is_some() {
            Ok(Instruction::transfer(TransferKind::Return, None))
        } else {
            let value = self.expression(0.0)?;
            Ok(Instruction::transfer(TransferKind::Return, Some(value)))
        }
    }

    /// A function definition becomes a binding of a function value.
    fn function_def(&mut self) -> Result<Instruction, String> {
        self.advance();
        let name = self.expect_word("after the function keyword")?;
        self.function_rest(name)
    }

    /// From the parameter list on: `( params ) [returns type] block`. A
    /// parameter is a name with an optional annotation or, with
    /// `stmt.let.type_first`, a type word and a name, where a type word
    /// alone (C's `void`) declares nothing.
    fn function_rest(&mut self, name: String) -> Result<Instruction, String> {
        let schema: &'a LanguageSchema = self.schema;
        let call = self.schema.structure.call.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.expect_op(&call.open, "after function name")?;
        let binding = schema.statements.binding.as_ref();
        let annotation: &[String] = binding.map_or(&[], |b| b.type_annotation.as_slice());
        let type_first = binding.map_or(false, |b| b.type_first);
        let mut params = Vec::new();
        while !self.is_op(&call.close) && !self.at_end() {
            if type_first {
                let type_word = self.expect_word("as a parameter type")?;
                if !binding.map_or(false, |b| spelled(&b.keyword, &type_word)) {
                    return Err(format!("'{}' is not a type word", type_word));
                }
                if self.peek().kind == Kind::Word {
                    params.push(self.advance().text);
                }
            } else {
                params.push(self.expect_word("as a parameter name")?);
                if self.peek().kind == Kind::Op && spelled(annotation, &self.peek().text) {
                    self.advance();
                    self.expect_word("as a type name")?;
                }
            }
            // Parameters are separated by the call separator or, between
            // typed groups (Pascal's `a: integer; b: real`), the terminator.
            if let Some(sep) = &call.separator {
                if self.is_op(sep) {
                    self.advance();
                }
            }
            if self.peek().kind == Kind::Op && schema.is_terminator(&self.peek().text) {
                self.advance();
            }
        }
        self.expect_op(&call.close, "after parameters")?;
        if self.peek().kind == Kind::Op && spelled(&schema.statements.function_returns, &self.peek().text) {
            self.advance();
            self.expect_word("as a return type")?;
        }
        // A header that ends with a terminator (Pascal's `;`) may be followed
        // by declarations before the body block; they open the body.
        let mut prelude = Vec::new();
        if self.peek().kind == Kind::Op && schema.is_terminator(&self.peek().text) {
            loop {
                self.skip_terminators();
                let declares = self.peek().kind == Kind::Word
                    && !type_first
                    && binding.map_or(false, |b| spelled(&b.keyword, &self.peek().text));
                if !declares {
                    break;
                }
                prelude.push(self.binding()?);
            }
        }
        let block = self.block()?;
        let body = if prelude.is_empty() {
            block
        } else {
            prelude.push(block);
            Instruction::Sequence(prelude)
        };
        let def = Function { name: name.clone(), params, body };
        Ok(Instruction::assign(name, Instruction::Literal(Value::Function(Rc::new(def)))))
    }

    fn assignment_or_expression(&mut self) -> Result<Instruction, String> {
        let expr = self.expression(0.0)?;
        if !self.at_assignment() {
            return Ok(expr);
        }
        let assign = self.advance().text;
        let value = self.expression(0.0)?;
        match expr {
            Instruction::Variable(name) => Ok(Instruction::assign(name, value)),
            Instruction::Operate { op: Op::Index, mut operands } if operands.len() == 2 => {
                let index = operands.pop().unwrap();
                match operands.pop().unwrap() {
                    Instruction::Variable(name) => Ok(Instruction::Assign {
                        target: Target::Index { name, index: Box::new(index) },
                        value: Box::new(value),
                    }),
                    _ => Err("Invalid assignment target".to_string()),
                }
            }
            _ => Err(format!("Invalid assignment target before '{}'", assign)),
        }
    }

    // ---------- expressions ----------

    fn expression(&mut self, min_prec: f32) -> Result<Instruction, String> {
        let mut left = self.prefix()?;
        loop {
            let tok = self.peek();
            if tok.kind != Kind::Op && tok.kind != Kind::Word {
                break;
            }
            let info = match self.schema.operators.binary.get(&tok.text).cloned() {
                Some(info) => info,
                None => break,
            };
            if info.precedence < min_prec {
                break;
            }
            self.advance();
            let next_min = if info.associativity == Assoc::Left { info.precedence + EPSILON } else { info.precedence };
            let right = self.expression(next_min)?;
            left = match info.op {
                // The pipe passes the left value as the first argument of the
                // call on the right; a bare name is a call with no other
                // arguments (`s.length` where the pipe is spelled `.`).
                Op::Pipe => match right {
                    Instruction::Invoke { function, mut args } => {
                        args.insert(0, left);
                        Instruction::Invoke { function, args }
                    }
                    Instruction::Variable(function) => Instruction::Invoke { function, args: vec![left] },
                    _ => return Err("Pipe operator requires a function call on the right side".to_string()),
                },
                op => Instruction::binary(op, left, right),
            };
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Instruction, String> {
        let tok = self.peek().clone();
        let schema: &'a LanguageSchema = self.schema;
        let structure = &schema.structure;

        // Unary operators (words such as `not` or symbols such as `-`).
        if tok.kind == Kind::Op || tok.kind == Kind::Word {
            if let Some(info) = schema.operators.unary.get(&tok.text).cloned() {
                self.advance();
                let operand = self.expression(info.precedence)?;
                return Ok(Instruction::unary(info.op, operand));
            }
        }

        let primary = match tok.kind {
            Kind::Number => {
                self.advance();
                Instruction::Literal(self.number_literal(&tok.text)?)
            }
            Kind::Str => {
                self.advance();
                Instruction::Literal(Value::String(tok.text))
            }
            Kind::Word => {
                let lits = &schema.literals;
                if lits.true_words.contains(&tok.text) {
                    self.advance();
                    Instruction::Literal(Value::Bool(true))
                } else if lits.false_words.contains(&tok.text) {
                    self.advance();
                    Instruction::Literal(Value::Bool(false))
                } else if lits.null_words.contains(&tok.text) {
                    self.advance();
                    Instruction::Literal(Value::Null)
                } else {
                    self.advance();
                    // A compound builtin name (println!, console.log) is one token.
                    let name = tok.text;
                    match structure.call.clone() {
                        Some(call) if self.is_op(&call.open) => {
                            self.advance();
                            let args = self.list(&call.close, call.separator.as_deref(), &structure.call_labels)?;
                            Instruction::Invoke { function: name, args }
                        }
                        _ => Instruction::Variable(name),
                    }
                }
            }
            Kind::Op => {
                if let Some(group) = structure.group.clone() {
                    if tok.text == group.open {
                        self.advance();
                        let inner = self.expression(0.0)?;
                        self.expect_op(&group.close, "to close a group")?;
                        return self.postfix(inner);
                    }
                }
                if let Some(array) = structure.array.clone() {
                    if tok.text == array.open {
                        self.advance();
                        let elements = self.list(&array.close, array.separator.as_deref(), &[])?;
                        return self.postfix(Instruction::Operate { op: Op::ArrayLiteral, operands: elements });
                    }
                }
                return Err(format!("Unexpected token: {}", tok.text));
            }
            Kind::Newline | Kind::Eof | Kind::Indent | Kind::Name => {
                return Err("Expected an expression".to_string());
            }
        };
        self.postfix(primary)
    }

    /// Postfix indexing: `expr[index]`, repeatable.
    fn postfix(&mut self, mut expr: Instruction) -> Result<Instruction, String> {
        let index_pair = match self.schema.structure.index.clone() {
            Some(p) => p,
            None => return Ok(expr),
        };
        while self.is_op(&index_pair.open) {
            self.advance();
            let index = self.expression(0.0)?;
            self.expect_op(&index_pair.close, "after array index")?;
            expr = Instruction::binary(Op::Index, expr, index);
        }
        Ok(expr)
    }

    /// Comma-separated expressions up to `close`, which is consumed. An
    /// item may carry an argument label (Swift's `f(n: 1)`), a word followed
    /// by one of `labels`; arguments pass by position, so it is dropped.
    fn list(&mut self, close: &str, separator: Option<&str>, labels: &[String]) -> Result<Vec<Instruction>, String> {
        let mut items = Vec::new();
        while !self.is_op(close) {
            if self.at_end() {
                return Err(format!("Expected '{}'", close));
            }
            let labelled = self.peek().kind == Kind::Word
                && self.peek_at(1).kind == Kind::Op
                && spelled(labels, &self.peek_at(1).text);
            if labelled {
                self.pos += 2;
            }
            items.push(self.expression(0.0)?);
            match separator {
                Some(sep) if self.is_op(sep) => {
                    self.advance();
                }
                _ => {}
            }
        }
        self.advance();
        Ok(items)
    }

    // ---------- number literals ----------

    fn number_literal(&self, text: &str) -> Result<Value, String> {
        let syntax = &self.schema.lexical.number;
        if let Some(prefix) = &syntax.hex_prefix {
            if let Some(digits) = text.strip_prefix(prefix.as_str()) {
                return BigInt::parse_bytes(digits.as_bytes(), 16)
                    .map(Value::Number)
                    .ok_or_else(|| format!("Invalid number: {}", text));
            }
        }
        if let Some(marker) = syntax.base_marker {
            if text.contains(marker) {
                let (numerator, denominator) = parse_base_n(text, marker, syntax.decimal_point, syntax.exponent_marker)?;
                return Ok(if denominator == BigInt::from(1) {
                    Value::Number(numerator)
                } else {
                    super::numeric::real(numerator, denominator, significant_figures(text))
                });
            }
        }
        if let Some(point) = syntax.decimal_point {
            if let Some(dot) = text.find(point) {
                let before = &text[..dot];
                let after = &text[dot + point.len_utf8()..];
                let denominator = BigInt::from(10).pow(after.len() as u32);
                let integer: BigInt = if before.is_empty() { BigInt::from(0) } else { parse_int(before, text)? };
                let fraction = parse_int(after, text)?;
                let numerator = integer * &denominator + fraction;
                return Ok(super::numeric::real(numerator, denominator, significant_figures(text)));
            }
        }
        Ok(Value::Number(parse_int(text, text)?))
    }
}

fn parse_int(digits: &str, whole: &str) -> Result<BigInt, String> {
    digits.parse::<BigInt>().map_err(|_| format!("Invalid number: {}", whole))
}

/// Significant figures of a decimal literal, with a floor of 15.
fn significant_figures(text: &str) -> usize {
    let digits: String = text.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let leading_zeros = digits.chars().take_while(|c| *c == '0').count();
    digits.len().saturating_sub(leading_zeros).max(1).max(15)
}

/// `<base>@<digits>[.<fraction>][^<exponent>]`, e.g. `16@FF`, `2@1011`, `10@1.5^3`.
fn parse_base_n(text: &str, marker: char, point: Option<char>, exponent: Option<char>) -> Result<(BigInt, BigInt), String> {
    let at = text.find(marker).ok_or_else(|| format!("Invalid base-N literal: missing '{}' in '{}'", marker, text))?;
    let base: u32 = text[..at]
        .parse()
        .map_err(|_| format!("Invalid base in literal '{}': base must be decimal integer", text))?;
    if !(2..=36).contains(&base) {
        return Err(format!("Invalid base {}: must be between 2 and 36", base));
    }
    let rest = &text[at + marker.len_utf8()..];
    if rest.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits after '{}'", text, marker));
    }
    let (mantissa, exp) = match exponent.and_then(|e| rest.find(e).map(|p| (p, e))) {
        Some((p, e)) => (&rest[..p], Some(&rest[p + e.len_utf8()..])),
        None => (rest, None),
    };
    let (int_part, frac_part) = match point.and_then(|d| mantissa.find(d).map(|p| (p, d))) {
        Some((p, d)) => (&mantissa[..p], Some(&mantissa[p + d.len_utf8()..])),
        None => (mantissa, None),
    };
    if int_part.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits", text));
    }
    let int_value = digits_in_base(int_part, base).map_err(|e| format!("Invalid base-N literal '{}': {}", text, e))?;
    let (mut numerator, denominator) = match frac_part {
        Some(frac) if !frac.is_empty() => {
            let frac_value = digits_in_base(frac, base).map_err(|e| format!("Invalid base-N literal '{}': {}", text, e))?;
            let scale = BigInt::from(base).pow(frac.len() as u32);
            (int_value * &scale + frac_value, scale)
        }
        Some(_) => return Err(format!("Invalid base-N literal '{}': missing digits after '.'", text)),
        None => (int_value, BigInt::from(1)),
    };
    if let Some(exp) = exp {
        if exp.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits after exponent marker", text));
        }
        let e = digits_in_base(exp, base)
            .map_err(|e| format!("Invalid base-N literal '{}': exponent {}", text, e))?
            .to_u32()
            .ok_or_else(|| format!("Invalid base-N literal '{}': exponent too large", text))?;
        numerator *= BigInt::from(base).pow(e);
    }
    Ok((numerator, denominator))
}

fn digits_in_base(digits: &str, base: u32) -> Result<BigInt, String> {
    let mut result = BigInt::from(0);
    for ch in digits.chars() {
        let value = match ch {
            '0'..='9' => ch as u32 - '0' as u32,
            'a'..='z' => ch as u32 - 'a' as u32 + 10,
            'A'..='Z' => ch as u32 - 'A' as u32 + 10,
            _ => return Err(format!("invalid digit '{}' for base {}", ch, base)),
        };
        if value >= base {
            return Err(format!("digit '{}' (value {}) is not valid in base {}", ch, value, base));
        }
        result = result * base + value;
    }
    Ok(result)
}

// ---------------- Postfix reduction ----------------
//
// A postfix language (RPLumen) is reduced onto the same instruction set.
// The stack is an ordinary array bound to a hidden name at the start of
// the program; a literal pushes onto it, an operator pops its operands
// into hidden temporaries and pushes the result, a control word takes its
// condition from the top, and a bare word either runs the program stored
// under it or pushes its value. Six internal invocations carry the stack
// mechanics; no definition can spell them.

/// The hidden binding holding the stack.
pub const STACK: &str = "#stack";

/// What a run of postfix words is part of, which says where it ends.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Run {
    Block,
    Program,
    Line,
}

fn push(value: Instruction) -> Instruction {
    Instruction::Invoke { function: "<push>".to_string(), args: vec![Instruction::Variable(STACK.to_string()), value] }
}

fn pop() -> Instruction {
    Instruction::Invoke { function: "<pop>".to_string(), args: vec![Instruction::Variable(STACK.to_string())] }
}

impl<'a> Parser<'a> {
    fn postfix_program(&mut self) -> Result<Instruction, String> {
        let stack = Instruction::assign(STACK.to_string(), Instruction::Literal(Value::Array(Vec::new())));
        let body = self.postfix_body(&[], Run::Block)?;
        if !self.at_end() {
            return Err(format!("Unexpected '{}'", self.peek().text));
        }
        Ok(Instruction::Sequence(vec![stack, body]))
    }

    /// Words up to one in `stops` (not consumed) or the end. A quoted
    /// name is held for the word after it, which must take one. What else
    /// ends the words is what they are part of: a block's end at its
    /// closing delimiter (an opener inside it is an indentation error), a
    /// program value's at its bracket (delimiters pass), a line's at the
    /// line end.
    fn postfix_body(&mut self, stops: &[String], run: Run) -> Result<Instruction, String> {
        let structure: &'a crate::schema::Structure = &self.schema.structure;
        let mut items = Vec::new();
        let mut name: Option<String> = None;
        loop {
            if run == Run::Line {
                while self.peek().kind == Kind::Op && self.schema.is_terminator(&self.peek().text) {
                    self.advance();
                }
                if self.peek().kind == Kind::Newline {
                    break;
                }
            } else {
                self.skip_terminators();
            }
            if self.at_end() || self.at_one_of(stops).is_some() {
                break;
            }
            let opener = self.at_one_of(&structure.block_open).is_some();
            if opener || self.at_one_of(&structure.block_close).is_some() {
                match run {
                    Run::Program => {
                        self.advance();
                        continue;
                    }
                    Run::Block if opener => return Err("Unexpected indentation".to_string()),
                    _ => break,
                }
            }
            let tok = self.advance();
            match tok.kind {
                Kind::Name => {
                    if name.is_some() {
                        return Err(format!("A name is already waiting before '{}'", tok.text));
                    }
                    name = Some(tok.text);
                }
                Kind::Number | Kind::Str if name.is_some() => {
                    return Err(format!("The name '{}' must be followed by a word that takes it", name.unwrap()));
                }
                Kind::Number => items.push(push(Instruction::Literal(self.number_literal(&tok.text)?))),
                Kind::Str => items.push(push(Instruction::Literal(Value::String(tok.text)))),
                Kind::Word => items.push(self.postfix_word(&tok.text, true, &mut name)?),
                Kind::Op => items.push(self.postfix_word(&tok.text, false, &mut name)?),
                Kind::Newline | Kind::Indent | Kind::Eof => unreachable!("terminators are skipped"),
            }
        }
        if let Some(name) = name {
            return Err(format!("The name '{}' has no word to take it", name));
        }
        Ok(Instruction::Sequence(items))
    }

    /// The body a control word governs, in the block style: between the
    /// delimiters synthesised from indentation or written as braces, or
    /// the words up to a closer.
    fn governed(&mut self) -> Result<Instruction, String> {
        let structure: &'a crate::schema::Structure = &self.schema.structure;
        match structure.blocks {
            BlockStyle::Indentation | BlockStyle::Braces => {
                self.skip_terminators();
                let i = self
                    .at_one_of(&structure.block_open)
                    .ok_or_else(|| format!("Expected an indented block, got '{}'", self.peek().text))?;
                self.advance();
                let close = std::slice::from_ref(&structure.block_close[i]);
                let body = self.postfix_body(close, Run::Block)?;
                self.expect_close(&close[0])?;
                Ok(body)
            }
            BlockStyle::Keyword => {
                let body = self.postfix_body(&structure.block_close, Run::Block)?;
                self.expect_any_close()?;
                Ok(body)
            }
        }
    }

    /// A loop's condition, run again each pass: the words up to where the
    /// body begins, which is the line end, the opener or the intro word.
    fn postfix_condition(&mut self) -> Result<Instruction, String> {
        let structure: &'a crate::schema::Structure = &self.schema.structure;
        match structure.blocks {
            BlockStyle::Indentation => self.postfix_body(&[], Run::Line),
            BlockStyle::Braces => self.postfix_body(&structure.block_open, Run::Block),
            BlockStyle::Keyword => {
                let condition = self.postfix_body(&structure.block_intro, Run::Block)?;
                self.expect_intro()?;
                Ok(condition)
            }
        }
    }

    /// Step onto an else word when one follows past line ends.
    fn take_else(&mut self, else_words: &[String]) -> bool {
        let mut ahead = 0;
        loop {
            let tok = &self.tokens[(self.pos + ahead).min(self.tokens.len() - 1)];
            let between = tok.kind == Kind::Newline || (tok.kind == Kind::Op && self.schema.is_terminator(&tok.text));
            if between {
                ahead += 1;
            } else if (tok.kind == Kind::Word || tok.kind == Kind::Op) && else_words.iter().any(|w| *w == tok.text) {
                self.pos += ahead + 1;
                return true;
            } else {
                return false;
            }
        }
    }

    fn expect_intro(&mut self) -> Result<(), String> {
        let structure: &'a crate::schema::Structure = &self.schema.structure;
        if self.at_one_of(&structure.block_intro).is_some() {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}', got '{}'", structure.block_intro[0], self.peek().text))
        }
    }

    /// Bind the top of the stack to a hidden temporary.
    fn take(&mut self, purpose: &str) -> (String, Instruction) {
        let name = self.hidden_name(purpose);
        let bind = Instruction::assign(name.clone(), pop());
        (name, bind)
    }

    /// One word of a postfix program; `is_word` tells a word from a symbol,
    /// and `name` is a quoted name waiting for a word that takes one,
    /// cleared when this word does.
    fn postfix_word(&mut self, word: &str, is_word: bool, name: &mut Option<String>) -> Result<Instruction, String> {
        let schema: &'a LanguageSchema = self.schema;
        let s = &schema.statements;
        let structure = &schema.structure;
        let stack = &schema.stack;
        let lits = &schema.literals;
        let closes = &structure.block_close;
        let variable = |n: &str| Instruction::Variable(n.to_string());

        // The word after a quoted name takes it.
        if let Some(taken) = name.take() {
            if spelled(&s.assignment, word) || s.binding.as_ref().map_or(false, |b| spelled(&b.keyword, word)) {
                return Ok(Instruction::assign(taken, pop()));
            }
            if s.loop_for.as_ref().map_or(false, |f| spelled(&f.keyword, word)) {
                let body = self.governed()?;
                let limit = self.hidden_name("end");
                let bind_limit = Instruction::assign(limit.clone(), pop());
                let bind_var = Instruction::assign(taken.clone(), pop());
                let condition = Instruction::binary(Op::Lt, variable(&taken), variable(&limit));
                let step = Instruction::assign(
                    taken.clone(),
                    Instruction::binary(Op::Add, variable(&taken), Instruction::Literal(Value::Number(BigInt::from(1)))),
                );
                return Ok(Instruction::Sequence(vec![
                    bind_limit,
                    bind_var,
                    Instruction::Loop { condition: Box::new(condition), body: Box::new(body), step: Some(Box::new(step)) },
                ]));
            }
            match schema.functions.get(word).copied() {
                Some(Builtin::Push) => {
                    let (value, bind_value) = self.take("value");
                    let call = Instruction::Invoke { function: word.to_string(), args: vec![variable(&taken), variable(&value)] };
                    return Ok(Instruction::Sequence(vec![bind_value, call]));
                }
                Some(Builtin::Put) => {
                    let (value, bind_value) = self.take("value");
                    let (index, bind_index) = self.take("index");
                    let call = Instruction::Invoke {
                        function: word.to_string(),
                        args: vec![variable(&taken), variable(&index), variable(&value)],
                    };
                    return Ok(Instruction::Sequence(vec![bind_value, bind_index, call]));
                }
                _ => return Err(format!("'{}' does not take a name, but '{}' was given", word, taken)),
            }
        }

        // Literal words.
        if lits.true_words.iter().any(|w| w == word) {
            return Ok(push(Instruction::Literal(Value::Bool(true))));
        }
        if lits.false_words.iter().any(|w| w == word) {
            return Ok(push(Instruction::Literal(Value::Bool(false))));
        }
        if lits.null_words.iter().any(|w| w == word) {
            return Ok(push(Instruction::Literal(Value::Null)));
        }

        // Control words: the condition is the top of the stack.
        if let Some(branch) = s.branch.as_ref().filter(|b| spelled(&b.keyword, word)) {
            // In the keyword style one closer ends both arms; otherwise
            // each arm is a governed block.
            let keyword = structure.blocks == BlockStyle::Keyword;
            let then_branch = if keyword {
                let mut stops = closes.clone();
                stops.extend(branch.else_keyword.iter().cloned());
                self.postfix_body(&stops, Run::Block)?
            } else {
                self.governed()?
            };
            let has_else = if keyword {
                self.at_one_of(&branch.else_keyword).is_some() && {
                    self.advance();
                    true
                }
            } else {
                self.take_else(&branch.else_keyword)
            };
            let else_branch = if has_else {
                Some(if keyword { self.postfix_body(closes, Run::Block)? } else { self.governed()? })
            } else {
                None
            };
            if keyword {
                self.expect_any_close()?;
            }
            return Ok(Instruction::Branch {
                condition: Box::new(pop()),
                then_branch: Box::new(then_branch),
                else_branch: else_branch.map(Box::new),
            });
        }
        if spelled(&s.loop_while, word) {
            let condition = self.postfix_condition()?;
            let body = self.governed()?;
            let condition = Instruction::Sequence(vec![condition, pop()]);
            return Ok(Instruction::Loop { condition: Box::new(condition), body: Box::new(body), step: None });
        }
        if spelled(&s.loop_until, word) {
            // Head first as in Lumen; the condition is tested after the body.
            let condition = self.postfix_condition()?;
            let body = self.governed()?;
            let stop = Instruction::Branch {
                condition: Box::new(Instruction::Sequence(vec![condition, pop()])),
                then_branch: Box::new(Instruction::transfer(TransferKind::Break, None)),
                else_branch: None,
            };
            return Ok(Instruction::Loop {
                condition: Box::new(Instruction::Literal(Value::Bool(true))),
                body: Box::new(body),
                step: Some(Box::new(stop)),
            });
        }
        if spelled(&s.return_, word) {
            return Ok(Instruction::transfer(TransferKind::Return, None));
        }
        if spelled(&s.break_, word) {
            return Ok(Instruction::transfer(TransferKind::Break, None));
        }
        if spelled(&s.continue_, word) {
            return Ok(Instruction::transfer(TransferKind::Continue, None));
        }

        // A program value: its body, reduced now, runs when a word names it.
        if let Some(i) = stack.program_open.iter().position(|o| o == word) {
            let close = std::slice::from_ref(&stack.program_close[i]);
            let body = self.postfix_body(close, Run::Program)?;
            self.expect_close(&close[0])?;
            let def = Function { name: "<program>".to_string(), params: Vec::new(), body };
            return Ok(push(Instruction::Literal(Value::Function(Rc::new(def)))));
        }

        // An array literal gathers what its body pushes.
        if let Some(array) = structure.array.as_ref().filter(|a| a.open == word) {
            let close = std::slice::from_ref(&array.close);
            let body = self.postfix_body(close, Run::Block)?;
            self.expect_close(&array.close)?;
            let mark = self.hidden_name("mark");
            let depth = Instruction::Invoke { function: "<depth>".to_string(), args: vec![variable(STACK)] };
            let gather = Instruction::Invoke { function: "<gather>".to_string(), args: vec![variable(STACK), variable(&mark)] };
            return Ok(Instruction::Sequence(vec![Instruction::assign(mark, depth), body, gather]));
        }

        // Stack words.
        if spelled(&stack.dup, word) {
            let (a, bind_a) = self.take("a");
            return Ok(Instruction::Sequence(vec![bind_a, push(variable(&a)), push(variable(&a))]));
        }
        if spelled(&stack.drop, word) {
            return Ok(pop());
        }
        if spelled(&stack.swap, word) {
            let (b, bind_b) = self.take("b");
            let (a, bind_a) = self.take("a");
            return Ok(Instruction::Sequence(vec![bind_b, bind_a, push(variable(&b)), push(variable(&a))]));
        }
        if spelled(&stack.over, word) {
            let (b, bind_b) = self.take("b");
            let (a, bind_a) = self.take("a");
            return Ok(Instruction::Sequence(vec![bind_b, bind_a, push(variable(&a)), push(variable(&b)), push(variable(&a))]));
        }
        if spelled(&stack.rot, word) {
            let (c, bind_c) = self.take("c");
            let (b, bind_b) = self.take("b");
            let (a, bind_a) = self.take("a");
            return Ok(Instruction::Sequence(vec![
                bind_c,
                bind_b,
                bind_a,
                push(variable(&b)),
                push(variable(&c)),
                push(variable(&a)),
            ]));
        }
        if spelled(&stack.eval, word) {
            return Ok(Instruction::Invoke { function: "<eval>".to_string(), args: vec![variable(STACK), pop()] });
        }

        // Operators: operands come off the stack, the result goes back.
        if let Some(info) = schema.operators.binary.get(word) {
            let (b, bind_b) = self.take("b");
            let (a, bind_a) = self.take("a");
            return Ok(Instruction::Sequence(vec![bind_b, bind_a, push(Instruction::binary(info.op, variable(&a), variable(&b)))]));
        }
        if let Some(info) = schema.operators.unary.get(word) {
            return Ok(push(Instruction::unary(info.op, pop())));
        }

        // Builtins: arguments come off the stack; a result goes back.
        if let Some(builtin) = schema.functions.get(word).copied() {
            let (arity, yields) = match builtin {
                Builtin::Emit | Builtin::PrintLine | Builtin::Write | Builtin::Error => (1, false),
                Builtin::CharAt | Builtin::Get | Builtin::Real => (2, true),
                Builtin::Push | Builtin::Put => return Err(format!("'{}' needs a quoted name before it", word)),
                Builtin::Extern | Builtin::Range => return Err(format!("'{}' has no postfix form", word)),
                Builtin::Pop | Builtin::Word | Builtin::Eval | Builtin::Depth | Builtin::Gather => {
                    unreachable!("internal builtins have no surface name")
                }
                _ => (1, true),
            };
            let mut items = Vec::new();
            let mut args = Vec::new();
            for n in 0..arity {
                let (arg, bind) = self.take(&format!("arg{}", arity - n));
                items.push(bind);
                args.push(variable(&arg));
            }
            args.reverse();
            let call = Instruction::Invoke { function: word.to_string(), args };
            items.push(if yields { push(call) } else { call });
            return Ok(Instruction::Sequence(items));
        }

        // Any other word: run the program bound to it, or push its value.
        if !is_word {
            return Err(format!("Unexpected '{}'", word));
        }
        Ok(Instruction::Invoke { function: "<word>".to_string(), args: vec![variable(STACK), variable(word)] })
    }
}
