// src/kernel/lexer.rs
//
// Pure lossless ASCII segmentation - kernel level.
// No semantic token classification. No language assumptions.
// Converts source text -> tokens (raw lexeme strings with position info).
// Language modules handle all interpretation of lexemes.
//
// ARCHITECTURE:
// - Token is just { lexeme: String } - no semantic categories
// - Lexer performs maximal-munch segmentation using language-provided multi-char sequences
// - All semantic interpretation (keywords, operators, types) happens in language layer

use crate::kernel::registry::{LumenResult, TokenRegistry};

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub lexeme: String,
}

impl Token {
    pub fn new(lexeme: String) -> Self {
        Self { lexeme }
    }
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

/// Tokenize source code using maximal-munch segmentation.
/// Language modules provide multi-char sequences via token_reg.
/// The lexer has NO semantic knowledge - it just segments the text.
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

        // whitespace (skip but track for position)
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // strings: "..."
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(format!("Unterminated string at line {line}"));
            }
            i += 1; // include closing quote
            let lexeme = s[start..i].to_string();
            out.push(SpannedToken::new(Token::new(lexeme), line, col));
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
            let lexeme = s[start..i].to_string();
            out.push(SpannedToken::new(Token::new(lexeme), line, col));
            continue;
        }

        // identifiers / keywords (maximal-munch for word-like sequences)
        if is_word_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_word_continue(bytes[i]) {
                i += 1;
            }
            let lexeme = s[start..i].to_string();
            out.push(SpannedToken::new(Token::new(lexeme), line, col));
            continue;
        }

        // multi-char sequences and single-char tokens
        // Try maximal-munch using language-provided multi-char lexemes
        let remaining = &s[i..];
        let mut matched = false;

        // Try multi-char sequences in descending length order
        for &multichar in token_reg.multichar_lexemes() {
            if remaining.starts_with(multichar) {
                let lexeme = multichar.to_string();
                out.push(SpannedToken::new(Token::new(lexeme), line, col));
                i += multichar.len();
                matched = true;
                break;
            }
        }

        if matched {
            continue;
        }

        // No multi-char match, emit single character
        let ch = bytes[i] as char;
        if ch.is_ascii_graphic() || ch.is_ascii_punctuation() {
            let lexeme = ch.to_string();
            out.push(SpannedToken::new(Token::new(lexeme), line, col));
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
