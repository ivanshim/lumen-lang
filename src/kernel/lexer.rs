// src/kernel/lexer.rs
//
// Pure lossless ASCII segmentation - kernel level.
// No semantic token classification. No language assumptions.
// Converts source text -> tokens (raw lexeme strings with byte span).
// Language modules handle all interpretation of lexemes.
//
// AUTHORITY:
// - Span { start, end } (byte offsets) is the AUTHORITATIVE source-location coordinate system
// - All parsing, AST construction, and evaluation logic must rely on Span
// - line/col are DIAGNOSTIC-ONLY (derived metadata, used only for error messages)
//
// ARCHITECTURE:
// - Token is { lexeme: String, span: Span } - no semantic categories
// - SpannedToken also carries line/col for diagnostic formatting only
// - Lexer performs maximal-munch segmentation using language-provided multi-char sequences
// - All semantic interpretation (keywords, operators, types) happens in language layer

use crate::kernel::registry::{LumenResult, TokenRegistry};

/// Explicit byte span: (start, end) offsets in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize, // inclusive byte offset
    pub end: usize,   // exclusive byte offset
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub lexeme: String,
    pub span: Span,
}

impl Token {
    pub fn new(lexeme: String, span: Span) -> Self {
        Self { lexeme, span }
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
    let bytes = source.as_bytes();
    let mut byte_pos = 0usize;
    let mut line_no = 1usize;
    let mut col_in_line = 1usize;

    while byte_pos < bytes.len() {
        // Skip whitespace and track line/column info
        if bytes[byte_pos].is_ascii_whitespace() {
            if bytes[byte_pos] == b'\n' {
                line_no += 1;
                col_in_line = 1;
            } else {
                col_in_line += 1;
            }
            byte_pos += 1;
            continue;
        }

        let start_byte = byte_pos;
        let start_col = col_in_line;

        // strings: "..."
        if bytes[byte_pos] == b'"' {
            let start = byte_pos;
            byte_pos += 1;
            col_in_line += 1;
            while byte_pos < bytes.len() && bytes[byte_pos] != b'"' {
                if bytes[byte_pos] == b'\n' {
                    line_no += 1;
                    col_in_line = 1;
                } else {
                    col_in_line += 1;
                }
                byte_pos += 1;
            }
            if byte_pos >= bytes.len() {
                return Err(format!("Unterminated string at line {line_no}"));
            }
            byte_pos += 1; // include closing quote
            col_in_line += 1;
            let lexeme = source[start..byte_pos].to_string();
            let span = Span::new(start, byte_pos);
            out.push(SpannedToken::new(Token::new(lexeme, span), line_no, start_col));
            continue;
        }

        // numbers: 123 or 123.45
        if bytes[byte_pos].is_ascii_digit() {
            let start = byte_pos;
            while byte_pos < bytes.len() && bytes[byte_pos].is_ascii_digit() {
                col_in_line += 1;
                byte_pos += 1;
            }
            if byte_pos < bytes.len() && bytes[byte_pos] == b'.' {
                col_in_line += 1;
                byte_pos += 1;
                while byte_pos < bytes.len() && bytes[byte_pos].is_ascii_digit() {
                    col_in_line += 1;
                    byte_pos += 1;
                }
            }
            let lexeme = source[start..byte_pos].to_string();
            let span = Span::new(start, byte_pos);
            out.push(SpannedToken::new(Token::new(lexeme, span), line_no, start_col));
            continue;
        }

        // identifiers / keywords (maximal-munch for word-like sequences)
        if is_word_start(bytes[byte_pos]) {
            let start = byte_pos;
            byte_pos += 1;
            col_in_line += 1;
            while byte_pos < bytes.len() && is_word_continue(bytes[byte_pos]) {
                col_in_line += 1;
                byte_pos += 1;
            }
            let lexeme = source[start..byte_pos].to_string();
            let span = Span::new(start, byte_pos);
            out.push(SpannedToken::new(Token::new(lexeme, span), line_no, start_col));
            continue;
        }

        // multi-char sequences and single-char tokens
        // Try maximal-munch using language-provided multi-char lexemes
        let remaining = &source[byte_pos..];
        let mut matched = false;

        // Try multi-char sequences in descending length order
        for &multichar in token_reg.multichar_lexemes() {
            if remaining.starts_with(multichar) {
                let lexeme = multichar.to_string();
                let span = Span::new(byte_pos, byte_pos + multichar.len());
                out.push(SpannedToken::new(Token::new(lexeme, span), line_no, start_col));
                col_in_line += multichar.len();
                byte_pos += multichar.len();
                matched = true;
                break;
            }
        }

        if matched {
            continue;
        }

        // No multi-char match, emit single character
        let ch = bytes[byte_pos] as char;
        if ch.is_ascii_graphic() || ch.is_ascii_punctuation() {
            let lexeme = ch.to_string();
            let span = Span::new(byte_pos, byte_pos + 1);
            out.push(SpannedToken::new(Token::new(lexeme, span), line_no, start_col));
            col_in_line += 1;
            byte_pos += 1;
            continue;
        }

        return Err(format!("Unexpected character '{ch}' at line {line_no}, col {start_col}"));
    }

    Ok(out)
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_word_continue(b: u8) -> bool {
    is_word_start(b) || b.is_ascii_digit()
}
