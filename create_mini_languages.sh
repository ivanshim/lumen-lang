#!/bin/bash

# This script creates all remaining language module files

# ============================================================================
# MINI-SH (Shell Script)
# ============================================================================

cat > /home/user/lumen-lang/src_mini_sh/structure/structural.rs << 'EOF'
// Mini-SH: Shell script structural syntax

use crate::framework::ast::{Program, StmtNode};
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{err_at, LumenResult, Registry};

pub const LPAREN: &str = "LPAREN";
pub const RPAREN: &str = "RPAREN";
pub const LBRACE: &str = "LBRACE";
pub const RBRACE: &str = "RBRACE";
pub const SEMICOLON: &str = "SEMICOLON";
pub const DOLLAR: &str = "DOLLAR";
pub const EOF: &str = "EOF";

pub fn consume_semicolons(parser: &mut Parser) {
    while matches!(parser.peek(), Token::Feature(k) if *k == SEMICOLON) {
        parser.advance();
    }
}

pub fn parse_block(parser: &mut Parser) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    match parser.advance() {
        Token::Feature(k) if k == LBRACE => {}
        _ => return Err(err_at(parser, "Expected '{'")),
    }
    let mut statements = Vec::new();
    consume_semicolons(parser);
    while !matches!(parser.peek(), Token::Feature(k) if *k == RBRACE || *k == EOF) {
        let stmt = parser.reg.find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;
        statements.push(stmt);
        consume_semicolons(parser);
    }
    match parser.advance() {
        Token::Feature(k) if k == RBRACE => {}
        _ => return Err(err_at(parser, "Expected '}'")),
    }
    Ok(statements)
}

pub fn parse_program(parser: &mut Parser) -> LumenResult<Program> {
    let mut statements = Vec::new();
    consume_semicolons(parser);
    while !matches!(parser.peek(), Token::Feature(k) if *k == EOF) {
        let stmt = parser.reg.find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;
        statements.push(stmt);
        consume_semicolons(parser);
    }
    Ok(Program::new(statements))
}

pub fn process_tokens(raw_tokens: Vec<crate::framework::lexer::SpannedToken>) -> LumenResult<Vec<crate::framework::lexer::SpannedToken>> {
    let mut tokens = raw_tokens;
    let line = tokens.last().map(|t| t.line).unwrap_or(1);
    tokens.push(crate::framework::lexer::SpannedToken {
        tok: Token::Feature(EOF),
        line,
        col: 1,
    });
    Ok(tokens)
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_single_char('(', LPAREN);
    reg.tokens.add_single_char(')', RPAREN);
    reg.tokens.add_single_char('{', LBRACE);
    reg.tokens.add_single_char('}', RBRACE);
    reg.tokens.add_single_char(';', SEMICOLON);
    reg.tokens.add_single_char('$', DOLLAR);
}
EOF

# Create all expression files for mini-sh
cat > /home/user/lumen-lang/src_mini_sh/expressions/literals.rs << 'EOF'
// Mini-SH: Literals
use crate::framework::ast::ExprNode;
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{ExprPrefix, LumenResult, Registry};
use crate::framework::runtime::{Env, Value};

pub const TRUE: &str = "TRUE";
pub const FALSE: &str = "FALSE";

#[derive(Debug)]
pub struct NumberLiteral { pub value: f64 }

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Value::Number(self.value))
    }
}

pub struct NumberLiteralPrefix;
impl ExprPrefix for NumberLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Number(_))
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Number(n) => Ok(Box::new(NumberLiteral { value: n })),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
struct BoolLiteral { value: bool }
impl ExprNode for BoolLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Value::Bool(self.value))
    }
}

pub struct BoolLiteralPrefix;
impl ExprPrefix for BoolLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(TRUE) | Token::Feature(FALSE))
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Feature(TRUE) => Ok(Box::new(BoolLiteral { value: true })),
            Token::Feature(FALSE) => Ok(Box::new(BoolLiteral { value: false })),
            _ => unreachable!(),
        }
    }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("true", TRUE);
    reg.tokens.add_keyword("false", FALSE);
    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
}
EOF

echo "Mini-SH files created successfully!"
