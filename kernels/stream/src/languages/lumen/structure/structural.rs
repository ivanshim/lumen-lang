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

// Array literals
pub const LBRACKET: &str = "[";
pub const RBRACKET: &str = "]";

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
///
/// Special handling: Newlines inside array literals (bracket depth > 0) are ignored,
/// allowing multiline array syntax. Newlines are treated as whitespace when inside brackets.
pub fn process_indentation(source: &str, raw_tokens: Vec<SpannedToken>) -> LumenResult<Vec<SpannedToken>> {
    let mut out = Vec::new();
    let mut indents = vec![0usize];
    let mut line_no = 1usize;
    let mut bracket_depth_global = 0i32;  // Track bracket depth across all lines

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

        // Skip comment-only lines (lines starting with # after indentation)
        if rest.starts_with('#') {
            line_no += 1;
            continue;
        }

        // Calculate bracket depth on this line to determine if we should handle indentation
        let mut bracket_depth = 0i32;
        let mut in_string_single = false;
        let mut in_string_double = false;
        let mut escape_next = false;

        for ch in rest.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            if ch == '\\' && (in_string_single || in_string_double) {
                escape_next = true;
                continue;
            }

            if ch == '\'' && !in_string_double {
                in_string_single = !in_string_single;
            } else if ch == '"' && !in_string_single {
                in_string_double = !in_string_double;
            } else if !in_string_single && !in_string_double {
                if ch == '[' {
                    bracket_depth += 1;
                } else if ch == ']' {
                    bracket_depth -= 1;
                }
            }
        }

        // Check if we're inside an array at the start of this line
        let inside_array = bracket_depth_global > 0;

        // Indentation handling (4-space indents for Lumen)
        // But skip indentation processing if we're inside an array literal
        if !inside_array {
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
        }

        // Add tokens from this line (from raw_tokens filtered by line number)
        // Filter out whitespace EXCEPT when inside string literals or arrays
        // The kernel lexer emits all characters including spaces, so we need to reconstruct
        // which spaces are part of strings vs which are separators
        let mut in_string_single = false;
        let mut in_string_double = false;
        let mut bracket_depth_line = bracket_depth_global;  // Start with global bracket depth

        for raw_tok in &raw_tokens {
            if raw_tok.line == line_no {
                let lexeme = &raw_tok.tok.lexeme;

                // Track bracket depth
                if lexeme == "[" && !in_string_single && !in_string_double {
                    bracket_depth_line += 1;
                    bracket_depth_global += 1;
                    out.push(raw_tok.clone());
                } else if lexeme == "]" && !in_string_single && !in_string_double {
                    bracket_depth_line -= 1;
                    bracket_depth_global -= 1;
                    out.push(raw_tok.clone());
                } else if lexeme == "'" && !in_string_double {
                    in_string_single = !in_string_single;
                    out.push(raw_tok.clone());
                } else if lexeme == "\"" && !in_string_single {
                    in_string_double = !in_string_double;
                    out.push(raw_tok.clone());
                } else if in_string_single || in_string_double {
                    // Inside a string - include everything, including whitespace
                    out.push(raw_tok.clone());
                } else if bracket_depth_line > 0 {
                    // Inside an array literal - include everything, including newlines and whitespace
                    // But skip the actual newline tokens (they're marked specially)
                    if lexeme == "\n" || lexeme == "\r" {
                        continue;  // Skip newline characters inside arrays - they're just whitespace
                    }
                    out.push(raw_tok.clone());
                } else {
                    // Outside both strings and arrays - filter whitespace tokens
                    if lexeme.len() == 1 {
                        let ch = lexeme.as_bytes()[0];
                        if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                            continue;  // Skip whitespace outside strings and arrays
                        }
                    }
                    out.push(raw_tok.clone());
                }
            }
        }

        // Add NEWLINE at end of line, but only if we're not inside an array literal
        if bracket_depth_global == 0 {
            out.push(SpannedToken {
                tok: Token::new(NEWLINE.to_string(), Span::new(0, 0)),
                line: line_no,
                col: spaces + rest.len() + 1,
            });
        }

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
        .with_literals(vec!["(", ")", "[", "]"])
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
