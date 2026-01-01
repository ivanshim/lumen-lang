// Mini-Rust structural tokens and parsing helpers

use crate::kernel::ast::{Program, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry};

// --------------------
// Mini-Rust Token Definitions
// --------------------

// Grouping
pub const LPAREN: &str = "LPAREN";
pub const RPAREN: &str = "RPAREN";
pub const LBRACE: &str = "LBRACE";
pub const RBRACE: &str = "RBRACE";

// Semicolon
pub const SEMICOLON: &str = "SEMICOLON";

// End of file
pub const EOF: &str = "EOF";

// --------------------
// Mini-Rust-specific Parsing Helpers
// --------------------

/// Consume newline tokens (for mini-rust compatibility with lumen style)
pub fn consume_newlines(parser: &mut Parser) {
    // Mini-rust doesn't use NEWLINE tokens like lumen, but we provide this for compatibility
    while matches!(parser.peek(), Token::Feature(SEMICOLON)) {
        parser.advance();
    }
}

/// Parse a block enclosed in curly braces
pub fn parse_block(parser: &mut Parser) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    let mut statements = Vec::new();

    // Expect '{'
    match parser.advance() {
        Token::Feature(k) if k == LBRACE => {}
        _ => return Err(err_at(parser, "Expected '{'")),
    }

    // Parse statements until '}'
    while !matches!(parser.peek(), Token::Feature(k) if *k == RBRACE || *k == EOF) {
        let stmt = parser
            .reg
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement in block"))?
            .parse(parser)?;

        statements.push(stmt);

        // Optionally consume semicolons
        while matches!(parser.peek(), Token::Feature(SEMICOLON)) {
            parser.advance();
        }
    }

    // Expect '}'
    match parser.advance() {
        Token::Feature(k) if k == RBRACE => Ok(statements),
        _ => Err(err_at(parser, "Expected '}'")),
    }
}

/// Parse the main program (sequence of statements)
pub fn parse_program(parser: &mut Parser) -> LumenResult<Program> {
    let mut statements = Vec::new();

    while !matches!(parser.peek(), Token::Feature(k) if *k == EOF) {
        let stmt = parser
            .reg
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;

        statements.push(stmt);

        // Optionally consume semicolons
        while matches!(parser.peek(), Token::Feature(SEMICOLON)) {
            parser.advance();
        }
    }

    Ok(Program::new(statements))
}

/// Add EOF token to raw tokens (no indentation processing for mini-rust)
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
    // Register structural tokens
    reg.tokens.add_single_char('(', LPAREN);
    reg.tokens.add_single_char(')', RPAREN);
    reg.tokens.add_single_char('{', LBRACE);
    reg.tokens.add_single_char('}', RBRACE);
    reg.tokens.add_single_char(';', SEMICOLON);
}
