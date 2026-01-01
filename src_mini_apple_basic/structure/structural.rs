// BASIC structural syntax (line numbers, GOTO)
use crate::kernel::ast::{Program, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry};

pub const LPAREN: &str = "(";
pub const RPAREN: &str = ")";
pub const LBRACE: &str = "{";
pub const RBRACE: &str = "}";
pub const SEMICOLON: &str = ";";
pub const EOF: &str = "EOF";

pub fn consume_semicolons(parser: &mut Parser) {
    while parser.peek().lexeme == SEMICOLON {
        parser.advance();
    }
}

pub fn parse_block(parser: &mut Parser) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    if parser.advance().lexeme != LBRACE {
        return Err(err_at(parser, "Expected '{'"));
    }
    let mut statements = Vec::new();
    consume_semicolons(parser);
    while !(parser.peek().lexeme == RBRACE || parser.peek().lexeme == EOF) {
        let stmt = parser.reg.find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;
        statements.push(stmt);
        consume_semicolons(parser);
    }
    if parser.advance().lexeme != RBRACE {
        return Err(err_at(parser, "Expected '}'"));
    }
    Ok(statements)
}

pub fn parse_program(parser: &mut Parser) -> LumenResult<Program> {
    let mut statements = Vec::new();
    consume_semicolons(parser);
    while parser.peek().lexeme != EOF {
        let stmt = parser.reg.find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;
        statements.push(stmt);
        consume_semicolons(parser);
    }
    Ok(Program::new(statements))
}

pub fn process_tokens(raw_tokens: Vec<crate::kernel::lexer::SpannedToken>) -> LumenResult<Vec<crate::kernel::lexer::SpannedToken>> {
    let mut tokens = raw_tokens;
    let line = tokens.last().map(|t| t.line).unwrap_or(1);
    tokens.push(crate::kernel::lexer::SpannedToken {
        tok: Token::new(EOF.to_string()),
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
