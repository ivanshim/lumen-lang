// src_lumen/structure/structural.rs
//
// Lumen structural tokens and language-specific parsing.
// Handles Python-style indentation: 4-space indents, INDENT/DEDENT tokens.
// Completely language-specific - ALL structural concepts defined here.

use crate::kernel::ast::{Program, StmtNode};
use crate::kernel::lexer::{Token, SpannedToken, Span};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::registry::{err_at, LumenResult};
use crate::languages::lumen::registry::Registry;

// --------------------
// Lumen Token Definitions (lexeme strings)
// --------------------

// Grouping
pub const LPAREN: &str = "(";
pub const RPAREN: &str = ")";

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
    while parser.peek().lexeme == NEWLINE {
        parser.advance();
    }
}

/// Parse Lumen block (indented statements) - Lumen-specific syntax handling.
pub fn parse_block(parser: &mut Parser, registry: &Registry) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    consume_newlines(parser);

    // Expect INDENT
    if parser.advance().lexeme != INDENT {
        return Err(err_at(parser, "Expected INDENT"));
    }

    consume_newlines(parser);

    let mut stmts = Vec::new();

    // Parse statements until DEDENT or EOF
    while parser.peek().lexeme != DEDENT && parser.peek().lexeme != EOF {
        let s = registry
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement in block"))?
            .parse(parser, registry)?;

        stmts.push(s);
        consume_newlines(parser);
    }

    // Expect DEDENT
    if parser.advance().lexeme != DEDENT {
        return Err(err_at(parser, "Expected DEDENT"));
    }

    Ok(stmts)
}

/// Lumen-specific program parsing with newline handling.
pub fn parse_program(parser: &mut Parser, registry: &Registry) -> LumenResult<Program> {
    let mut stmts = Vec::new();
    consume_newlines(parser);

    while parser.peek().lexeme != EOF {
        let stmt = registry
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser, registry)?;

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
                tok: Token::new(INDENT.to_string(), Span::new(0, 0)),
                line: line_no,
                col: 1,
            });
        } else if spaces < current {
            while *indents.last().unwrap() > spaces {
                indents.pop();
                out.push(SpannedToken {
                    tok: Token::new(DEDENT.to_string(), Span::new(0, 0)),
                    line: line_no,
                    col: 1,
                });
            }
            if *indents.last().unwrap() != spaces {
                return Err(format!("Indentation mismatch at line {line_no}"));
            }
        }

        // Add tokens from this line (from raw_tokens filtered by line number)
        // IMPORTANT: Filter out single-character whitespace tokens
        // The kernel lexer is now fully agnostic and emits all characters (including spaces, tabs, newlines)
        // Lumen's indentation processing needs only the meaningful tokens
        for raw_tok in &raw_tokens {
            if raw_tok.line == line_no {
                // Skip whitespace tokens (single-char spaces, tabs, newlines, carriage returns)
                if raw_tok.tok.lexeme.len() == 1 {
                    let ch = raw_tok.tok.lexeme.as_bytes()[0];
                    if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                        continue;
                    }
                }
                out.push(raw_tok.clone());
            }
        }

        // Add NEWLINE at end of line
        out.push(SpannedToken {
            tok: Token::new(NEWLINE.to_string(), Span::new(0, 0)),
            line: line_no,
            col: spaces + rest.len() + 1,
        });

        line_no += 1;
    }

    // Generate remaining DEDENT tokens
    while indents.len() > 1 {
        indents.pop();
        out.push(SpannedToken {
            tok: Token::new(DEDENT.to_string(), Span::new(0, 0)),
            line: line_no,
            col: 1,
        });
    }

    // Add EOF token
    out.push(SpannedToken {
        tok: Token::new(EOF.to_string(), Span::new(0, 0)),
        line: line_no,
        col: 1,
    });

    Ok(out)
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["(", ")"])
        .with_structural(vec!["newline", "indent", "dedent", "eof"])
}

// --------------------
// Registration
// --------------------

pub fn register(_reg: &mut Registry) {
    // No token registration needed - kernel lexer handles all segmentation
    // Parentheses are single-char lexemes emitted automatically
    // NEWLINE, INDENT, DEDENT, EOF tokens are generated by process_indentation()
}
