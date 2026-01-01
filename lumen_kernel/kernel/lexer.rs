// src/framework/lexer.rs
//
// Pure lexical analysis - framework level.
// No indentation handling. No language assumptions.
// Converts source text -> tokens (strings, numbers, identifiers, operators).
// Language modules handle structural tokens (INDENT/DEDENT, NEWLINE, EOF, parens, etc.)

use crate::kernel::registry::{LumenResult, TokenRegistry};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Data-carrying tokens (carry runtime values)
    Ident(String),
    Number(f64),
    String(String),

    // ALL other tokens (operators, keywords, structural elements)
    // are defined by modules as &'static str constants
    Feature(&'static str),
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub tok: Token,
    pub line: usize,
    pub col: usize,
}

impl SpannedToken {
    fn new(tok: Token, line: usize, col: usize) -> Self {
        Self { tok, line, col }
    }
}

/// Tokenize source code without any indentation or structural processing.
/// Language modules are responsible for adding INDENT/DEDENT/NEWLINE/EOF tokens.
pub fn lex(source: &str, token_reg: &TokenRegistry) -> LumenResult<Vec<SpannedToken>> {
    let mut out = Vec::new();
    let mut line_no = 1usize;

    for raw in source.lines() {
        lex_line(raw, line_no, 1, token_reg, &mut out)?;
        line_no += 1;
    }

    Ok(out)
}

fn lex_line(s: &str, line: usize, base_col: usize, token_reg: &TokenRegistry, out: &mut Vec<SpannedToken>) -> LumenResult<()> {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        let col = base_col + i;

        // whitespace (skip)
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // strings: "..."
        if bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(format!("Unterminated string at line {line}"));
            }
            let val = &s[start..i];
            i += 1;
            out.push(SpannedToken::new(Token::String(val.to_string()), line, col));
            continue;
        }

        // numbers: 123 or 123.45
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let n: f64 = s[start..i].parse().unwrap();
            out.push(SpannedToken::new(Token::Number(n), line, col));
            continue;
        }

        // identifiers / keywords
        if is_word_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_word_continue(bytes[i]) {
                i += 1;
            }
            let word = &s[start..i];
            let tok = token_reg.lookup_keyword(word)
                .unwrap_or_else(|| Token::Ident(word.to_string()));
            out.push(SpannedToken::new(tok, line, col));
            continue;
        }

        // two-char operators
        if i + 1 < bytes.len() {
            let two = &s[i..i + 2];
            if let Some(tok) = token_reg.lookup_two_char(two) {
                out.push(SpannedToken::new(tok, line, col));
                i += 2;
                continue;
            }
        }

        // single-char operators / punctuation
        let ch = bytes[i] as char;

        // Try to lookup operator in registry (no hardcoded tokens)
        if let Some(t) = token_reg.lookup_single_char(ch) {
            out.push(SpannedToken::new(t, line, col));
            i += 1;
            continue;
        }

        return Err(format!("Unexpected character '{ch}' at line {line}, col {col}"));
    }

    Ok(())
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_word_continue(b: u8) -> bool {
    is_word_start(b) || b.is_ascii_digit()
}
