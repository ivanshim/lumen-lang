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
use crate::schema::{spelled, Assoc, LanguageSchema, Op};

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
        let mut stmts = Vec::new();
        self.skip_terminators();
        while !self.at_end() {
            stmts.push(self.statement()?);
            self.skip_terminators();
        }
        Ok(Instruction::Sequence(stmts))
    }

    fn block(&mut self) -> Result<Instruction, String> {
        let open = self.schema.structure.block_open.clone();
        let close = self.schema.structure.block_close.clone();
        self.skip_terminators();
        self.expect_op(&open, "to open a block")?;
        let mut stmts = Vec::new();
        self.skip_terminators();
        while !self.is_op(&close) && !self.at_end() {
            stmts.push(self.statement()?);
            self.skip_terminators();
        }
        self.expect_op(&close, "to close a block")?;
        Ok(Instruction::Sequence(stmts))
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
        self.assignment_or_expression()
    }

    fn binding(&mut self) -> Result<Instruction, String> {
        let schema: &'a LanguageSchema = self.schema;
        let binding = schema.statements.binding.as_ref().expect("binding form present");
        self.advance(); // keyword
        if self.peek().kind == Kind::Word && spelled(&binding.mutable_modifier, &self.peek().text) {
            self.advance();
        }
        let name = self.expect_word("after the binding keyword")?;
        if self.peek().kind == Kind::Op && spelled(&binding.type_annotation, &self.peek().text) {
            self.advance();
            self.expect_word("as a type name")?;
        }
        self.expect_assignment("in a binding")?;
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
        self.advance(); // if / elif
        let condition = self.expression(0.0)?;
        let then_branch = self.block()?;

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
            } else {
                Some(self.block()?)
            }
        } else {
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

        let range = self.hidden_name("range");
        let bind_range = Instruction::assign(range.clone(), iterable);
        let bind_var =
            Instruction::assign(var.clone(), Instruction::unary(Op::RangeStart, Instruction::Variable(range.clone())));
        let condition = Instruction::binary(
            Op::Lt,
            Instruction::Variable(var.clone()),
            Instruction::unary(Op::RangeEnd, Instruction::Variable(range)),
        );
        let step = Instruction::assign(
            var.clone(),
            Instruction::binary(Op::Add, Instruction::Variable(var), Instruction::Literal(Value::Number(BigInt::from(1)))),
        );
        Ok(Instruction::Sequence(vec![
            bind_range,
            bind_var,
            Instruction::Loop { condition: Box::new(condition), body: Box::new(body), step: Some(Box::new(step)) },
        ]))
    }

    fn return_stmt(&mut self) -> Result<Instruction, String> {
        self.advance();
        let close = self.schema.structure.block_close.clone();
        if self.is_terminator() || self.at_end() || self.is_op(&close) {
            Ok(Instruction::transfer(TransferKind::Return, None))
        } else {
            let value = self.expression(0.0)?;
            Ok(Instruction::transfer(TransferKind::Return, Some(value)))
        }
    }

    /// A function definition becomes a binding of a function value.
    fn function_def(&mut self) -> Result<Instruction, String> {
        let schema: &'a LanguageSchema = self.schema;
        self.advance();
        let name = self.expect_word("after the function keyword")?;
        let call = self.schema.structure.call.clone().ok_or_else(|| "This language has no call syntax".to_string())?;
        self.expect_op(&call.open, "after function name")?;
        let annotation: &[String] =
            schema.statements.binding.as_ref().map_or(&[], |b| b.type_annotation.as_slice());
        let mut params = Vec::new();
        while !self.is_op(&call.close) && !self.at_end() {
            params.push(self.expect_word("as a parameter name")?);
            if self.peek().kind == Kind::Op && spelled(annotation, &self.peek().text) {
                self.advance();
                self.expect_word("as a type name")?;
            }
            if let Some(sep) = &call.separator {
                if self.is_op(sep) {
                    self.advance();
                }
            }
        }
        self.expect_op(&call.close, "after parameters")?;
        if self.peek().kind == Kind::Op && spelled(&schema.statements.function_returns, &self.peek().text) {
            self.advance();
            self.expect_word("as a return type")?;
        }
        let body = self.block()?;
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
                Op::Pipe => match right {
                    Instruction::Invoke { function, mut args } => {
                        args.insert(0, left);
                        Instruction::Invoke { function, args }
                    }
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
                    let mut name = tok.text;
                    if self.peek().kind == Kind::Op {
                        let joined = format!("{}{}", name, self.peek().text);
                        if schema.functions.contains_key(&joined) {
                            self.advance();
                            name = joined;
                        }
                    }
                    match structure.call.clone() {
                        Some(call) if self.is_op(&call.open) => {
                            self.advance();
                            let args = self.list(&call.close, call.separator.as_deref())?;
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
                        let elements = self.list(&array.close, array.separator.as_deref())?;
                        return self.postfix(Instruction::Operate { op: Op::ArrayLiteral, operands: elements });
                    }
                }
                return Err(format!("Unexpected token: {}", tok.text));
            }
            Kind::Newline | Kind::Eof | Kind::Indent => {
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

    /// Comma-separated expressions up to `close`, which is consumed.
    fn list(&mut self, close: &str, separator: Option<&str>) -> Result<Vec<Instruction>, String> {
        let mut items = Vec::new();
        while !self.is_op(close) {
            if self.at_end() {
                return Err(format!("Expected '{}'", close));
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
                    Value::Real { numerator, denominator, precision: significant_figures(text) }
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
                return Ok(Value::Real { numerator, denominator, precision: significant_figures(text) });
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
