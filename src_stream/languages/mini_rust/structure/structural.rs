// Mini-Rust structural tokens and parsing helpers

use crate::src_stream::kernel::ast::{Program, StmtNode};
use crate::src_stream::kernel::lexer::{Token, SpannedToken, Span};
use crate::src_stream::kernel::parser::Parser;
use crate::src_stream::kernel::registry::{err_at, LumenResult, Registry};

// --------------------
// Mini-Rust Token Definitions
// --------------------

// Grouping
pub const LPAREN: &str = "(";
pub const RPAREN: &str = ")";
pub const LBRACE: &str = "{";
pub const RBRACE: &str = "}";

// Semicolon
pub const SEMICOLON: &str = ";";

// End of file
pub const EOF: &str = "EOF";

// --------------------
// Mini-Rust-specific Parsing Helpers
// --------------------

/// Consume newline tokens (for mini-rust compatibility with lumen style)
pub fn consume_newlines(parser: &mut Parser) {
    // Mini-rust doesn't use NEWLINE tokens like lumen, but we provide this for compatibility
    while parser.peek().lexeme == SEMICOLON {
        parser.advance();
    }
}

/// Parse a block enclosed in curly braces
pub fn parse_block(parser: &mut Parser) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    let mut statements = Vec::new();

    // Expect '{'
    if parser.advance().lexeme != LBRACE {
        return Err(err_at(parser, "Expected '{'"));
    }
    parser.skip_whitespace();

    // Parse statements until '}'
    while !(parser.peek().lexeme == RBRACE || parser.peek().lexeme == EOF) {
        parser.skip_whitespace();

        if parser.peek().lexeme == RBRACE || parser.peek().lexeme == EOF {
            break;
        }

        let stmt = parser
            .reg
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement in block"))?
            .parse(parser)?;

        statements.push(stmt);

        // Optionally consume semicolons and whitespace
        while parser.peek().lexeme == SEMICOLON {
            parser.advance();
            parser.skip_whitespace();
        }
        parser.skip_whitespace();
    }

    // Expect '}'
    if parser.advance().lexeme != RBRACE {
        return Err(err_at(parser, "Expected '}'"));
    }
    Ok(statements)
}

/// Parse the main program (sequence of statements)
pub fn parse_program(parser: &mut Parser) -> LumenResult<Program> {
    let mut statements = Vec::new();

    while parser.peek().lexeme != EOF {
        parser.skip_whitespace();

        if parser.peek().lexeme == EOF {
            break;
        }

        let stmt = parser
            .reg
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;

        statements.push(stmt);

        // Optionally consume semicolons and whitespace
        while parser.peek().lexeme == SEMICOLON {
            parser.advance();
            parser.skip_whitespace();
        }
        parser.skip_whitespace();
    }

    Ok(Program::new(statements))
}

/// Add EOF token to raw tokens (no indentation processing for mini-rust)
pub fn process_tokens(raw_tokens: Vec<crate::src_stream::kernel::lexer::SpannedToken>) -> LumenResult<Vec<crate::src_stream::kernel::lexer::SpannedToken>> {
    let mut tokens = raw_tokens;
    let line = tokens.last().map(|t| t.line).unwrap_or(1);
    tokens.push(crate::src_stream::kernel::lexer::SpannedToken {
        tok: Token::new(EOF.to_string(), Span::new(0, 0)),
        line,
        col: 1,
    });
    Ok(tokens)
}

// --------------------
// Registration
// --------------------

pub fn register(_reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
}
