// src_mini_php/structure/structural.rs
//
// Mini-PHP structural tokens and language-specific parsing.
// PHP-style: curly braces, semicolons, $ for variables

use crate::kernel::ast::{Program, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry};

// --------------------
// Mini-PHP Token Definitions
// --------------------

// Grouping
pub const LPAREN: &str = "LPAREN";
pub const RPAREN: &str = "RPAREN";
pub const LBRACE: &str = "LBRACE";
pub const RBRACE: &str = "RBRACE";

// Structural
pub const SEMICOLON: &str = "SEMICOLON";
pub const DOLLAR: &str = "DOLLAR";

// End of file
pub const EOF: &str = "EOF";

// --------------------
// Mini-PHP Parsing Helpers
// --------------------

/// Consume optional semicolons
pub fn consume_semicolons(parser: &mut Parser) {
    while matches!(parser.peek(), Token::Feature(k) if *k == SEMICOLON) {
        parser.advance();
    }
}

/// Parse a block enclosed in curly braces
pub fn parse_block(parser: &mut Parser) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    match parser.advance() {
        Token::Feature(k) if k == LBRACE => {}
        _ => return Err(err_at(parser, "Expected '{'")),
    }

    let mut statements = Vec::new();

    consume_semicolons(parser);
    while !matches!(parser.peek(), Token::Feature(k) if *k == RBRACE || *k == EOF) {
        let stmt = parser
            .reg
            .find_stmt(parser)
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

/// Parse the main program (sequence of statements)
pub fn parse_program(parser: &mut Parser) -> LumenResult<Program> {
    let mut statements = Vec::new();

    consume_semicolons(parser);
    while !matches!(parser.peek(), Token::Feature(k) if *k == EOF) {
        let stmt = parser
            .reg
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;

        statements.push(stmt);
        consume_semicolons(parser);
    }

    Ok(Program::new(statements))
}

/// Add EOF token to raw tokens (no indentation processing for PHP)
pub fn process_tokens(raw_tokens: Vec<crate::kernel::lexer::SpannedToken>) -> LumenResult<Vec<crate::kernel::lexer::SpannedToken>> {
    let mut tokens = raw_tokens;
    let line = tokens.last().map(|t| t.line).unwrap_or(1);
    tokens.push(crate::kernel::lexer::SpannedToken {
        tok: Token::Feature(EOF),
        line,
        col: 1,
    });
    Ok(tokens)
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register PHP's operator tokens
    reg.tokens.add_single_char('(', LPAREN);
    reg.tokens.add_single_char(')', RPAREN);
    reg.tokens.add_single_char('{', LBRACE);
    reg.tokens.add_single_char('}', RBRACE);
    reg.tokens.add_single_char(';', SEMICOLON);
    reg.tokens.add_single_char('$', DOLLAR);
}
