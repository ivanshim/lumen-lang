// src_lumen/structure/structural.rs
//
// Lumen structural tokens and language-specific parsing.
// Handles Python-style indentation: 4-space indents, INDENT/DEDENT tokens.
// Completely language-specific - ALL structural concepts defined here.

use crate::framework::ast::{Program, StmtNode};
use crate::framework::lexer::{Token, SpannedToken};
use crate::framework::parser::Parser;
use crate::framework::registry::{err_at, LumenResult, Registry};

// --------------------
// Lumen Token Definitions
// --------------------

// Grouping
pub const LPAREN: &str = "LPAREN";
pub const RPAREN: &str = "RPAREN";

// Layout (Python-style indentation)
pub const NEWLINE: &str = "NEWLINE";
pub const INDENT: &str = "INDENT";
pub const DEDENT: &str = "DEDENT";

// End of file
pub const EOF: &str = "EOF";

// --------------------
// Structural Tokens Configuration
// --------------------

/// Lumen's structural tokens - COMPLETELY language-specific.
/// Framework has zero knowledge of these.
#[derive(Clone, Copy)]
pub struct StructuralTokens {
    pub newline: &'static str,
    pub indent: &'static str,
    pub dedent: &'static str,
    pub eof: &'static str,
}

impl StructuralTokens {
    pub fn lumen() -> Self {
        StructuralTokens {
            newline: NEWLINE,
            indent: INDENT,
            dedent: DEDENT,
            eof: EOF,
        }
    }
}

// --------------------
// Lumen-specific Parsing Helpers
// --------------------

/// Consume newline tokens - Lumen-specific syntax handling.
pub fn consume_newlines(parser: &mut Parser) {
    while matches!(parser.peek(), Token::Feature(k) if *k == NEWLINE) {
        parser.advance();
    }
}

/// Parse Lumen block (indented statements) - Lumen-specific syntax handling.
pub fn parse_block(parser: &mut Parser) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    consume_newlines(parser);

    // Expect INDENT
    match parser.advance() {
        Token::Feature(k) if k == INDENT => {}
        _ => return Err(err_at(parser, "Expected INDENT")),
    }

    consume_newlines(parser);

    let mut stmts = Vec::new();

    // Parse statements until DEDENT or EOF
    while !matches!(parser.peek(), Token::Feature(k) if *k == DEDENT || *k == EOF) {
        let s = parser
            .reg
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement in block"))?
            .parse(parser)?;

        stmts.push(s);
        consume_newlines(parser);
    }

    // Expect DEDENT
    match parser.advance() {
        Token::Feature(k) if k == DEDENT => Ok(stmts),
        _ => Err(err_at(parser, "Expected DEDENT")),
    }
}

/// Lumen-specific program parsing with newline handling.
pub fn parse_program(parser: &mut Parser) -> LumenResult<Program> {
    let mut stmts = Vec::new();
    consume_newlines(parser);

    while !matches!(parser.peek(), Token::Feature(k) if *k == EOF) {
        let stmt = parser
            .reg
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;

        stmts.push(stmt);
        consume_newlines(parser);
    }

    Ok(Program::new(stmts))
}

// --------------------
// Indentation Processing
// --------------------

/// Post-process raw tokens to add indentation-based tokens.
/// Takes tokens from framework lexer (no INDENT/DEDENT/NEWLINE/EOF)
/// and produces final token stream for Lumen (with all structural tokens).
pub fn process_indentation(source: &str, raw_tokens: Vec<SpannedToken>) -> LumenResult<Vec<SpannedToken>> {
    let mut out = Vec::new();
    let mut indents = vec![0usize];
    let mut line_no = 1usize;

    for raw in source.lines() {
        // Count leading spaces
        let mut spaces = 0usize;
        for ch in raw.chars() {
            if ch == ' ' {
                spaces += 1;
            } else {
                break;
            }
        }

        let rest = &raw[spaces..];

        // Skip blank / whitespace-only lines (do not emit NEWLINE)
        if rest.trim().is_empty() {
            line_no += 1;
            continue;
        }

        // Indentation handling (4-space indents for Lumen)
        let current = *indents.last().unwrap();
        if spaces > current {
            if (spaces - current) % 4 != 0 {
                return Err(format!("Invalid indentation at line {line_no}"));
            }
            indents.push(spaces);
            out.push(SpannedToken {
                tok: Token::Feature(INDENT),
                line: line_no,
                col: 1,
            });
        } else if spaces < current {
            while *indents.last().unwrap() > spaces {
                indents.pop();
                out.push(SpannedToken {
                    tok: Token::Feature(DEDENT),
                    line: line_no,
                    col: 1,
                });
            }
            if *indents.last().unwrap() != spaces {
                return Err(format!("Indentation mismatch at line {line_no}"));
            }
        }

        // Add tokens from this line (from raw_tokens filtered by line number)
        for raw_tok in &raw_tokens {
            if raw_tok.line == line_no {
                out.push(raw_tok.clone());
            }
        }

        // Add NEWLINE at end of line
        out.push(SpannedToken {
            tok: Token::Feature(NEWLINE),
            line: line_no,
            col: spaces + rest.len() + 1,
        });

        line_no += 1;
    }

    // Generate remaining DEDENT tokens
    while indents.len() > 1 {
        indents.pop();
        out.push(SpannedToken {
            tok: Token::Feature(DEDENT),
            line: line_no,
            col: 1,
        });
    }

    // Add EOF token
    out.push(SpannedToken {
        tok: Token::Feature(EOF),
        line: line_no,
        col: 1,
    });

    Ok(out)
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register Lumen's operator tokens
    // Parentheses as single-char operators
    reg.tokens.add_single_char('(', LPAREN);
    reg.tokens.add_single_char(')', RPAREN);

    // Note: NEWLINE, INDENT, DEDENT, EOF tokens are generated by process_indentation()
    // and passed to parser via structural tokens config.
    // Framework no longer needs to know about them!
}
