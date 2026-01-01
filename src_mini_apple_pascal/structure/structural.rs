// Pascal structural syntax (BEGIN/END blocks)
use crate::kernel::ast::{Program, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry};

pub const LPAREN: &str = "LPAREN";
pub const RPAREN: &str = "RPAREN";
pub const BEGIN: &str = "BEGIN";
pub const END: &str = "END";
pub const SEMICOLON: &str = "SEMICOLON";
pub const EOF: &str = "EOF";

pub const LBRACE: &str = "BEGIN";  // Alias for compatibility
pub const RBRACE: &str = "END";    // Alias for compatibility

pub fn consume_semicolons(parser: &mut Parser) {
    while matches!(parser.peek(), Token::Feature(k) if *k == SEMICOLON) {
        parser.advance();
    }
}

pub fn parse_block(parser: &mut Parser) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    match parser.advance() {
        Token::Feature(k) if k == BEGIN => {}
        _ => return Err(err_at(parser, "Expected 'BEGIN'")),
    }
    let mut statements = Vec::new();
    consume_semicolons(parser);
    while !matches!(parser.peek(), Token::Feature(k) if *k == END || *k == EOF) {
        let stmt = parser.reg.find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;
        statements.push(stmt);
        consume_semicolons(parser);
    }
    match parser.advance() {
        Token::Feature(k) if k == END => {}
        _ => return Err(err_at(parser, "Expected 'END'")),
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

pub fn register(reg: &mut Registry) {
    reg.tokens.add_single_char('(', LPAREN);
    reg.tokens.add_single_char(')', RPAREN);
    reg.tokens.add_keyword("BEGIN", BEGIN);
    reg.tokens.add_keyword("END", END);
    reg.tokens.add_single_char(';', SEMICOLON);
}
