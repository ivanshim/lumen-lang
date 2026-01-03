// src/kernel/lexer.rs
//
// ONTOLOGICALLY NEUTRAL LEXER
//
// Pure lossless ASCII segmentation - kernel level.
// Zero semantic assumptions. Converts source bytes -> tokens (raw lexeme strings with span).
// All interpretation of lexemes (keywords, operators, numbers, strings, etc.) happens
// entirely in language modules.
//
// PRINCIPLE: The kernel lexer does not know what anything means.
// It only guarantees:
//   1. Lossless segmentation: every input byte → part of exactly one token
//   2. Maximal-munch: longest registered multi-char sequence is always preferred
//   3. Fallback to single-char: if no multi-char matches, emit one byte as token
//   4. Position tracking: line/col/span preserved for all tokens (including whitespace)
//
// AUTHORITY:
// - Span { start, end } (byte offsets) is AUTHORITATIVE source-location coordinate
// - All parsing, AST construction, evaluation use Span
// - line/col are DIAGNOSTIC-ONLY (derived metadata for error messages only)
//
// ARCHITECTURE:
// - Token: { lexeme: String, span: Span } - opaque, no semantic categories
// - SpannedToken: adds line/col for diagnostic formatting
// - Lexer: pure maximal-munch with language-provided sequences + single-char fallback
// - No character-class checks (no is_digit, is_alpha, is_whitespace)
// - No special-case handling (no string literals, numbers, identifiers, keywords)
// - No assumptions about human language conventions
//
// EXAMPLE:
// Input: "@@@if@@@then@@else"
// The lexer tokenizes this WITHOUT KNOWING what it means.
// All meaning is defined by the language module via registry and parser.

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

/// Tokenize source code using pure maximal-munch segmentation.
/// Language modules provide multi-char sequences via token_reg.
/// Kernel has ZERO semantic knowledge.
///
/// Algorithm:
///   1. At each byte position, try to match the longest language-supplied multi-char sequence
///   2. If no multi-char match, emit a single byte as a token (including all whitespace)
///   3. Track line/col for every byte (required for error reporting)
///   4. Never reject any input - all bytes are valid
///
/// This lexer makes NO assumptions about:
///   - What constitutes whitespace or if it's meaningful
///   - What constitutes identifiers, numbers, strings, keywords
///   - What characters are "allowed" or "unexpected"
///   - Human language conventions
///
/// All such interpretation is delegated entirely to language modules.
pub fn lex(source: &str, token_reg: &TokenRegistry) -> LumenResult<Vec<SpannedToken>> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut byte_pos = 0usize;
    let mut line_no = 1usize;
    let mut col_in_line = 1usize;

    while byte_pos < bytes.len() {
        let start_col = col_in_line;

        // Try maximal-munch: match longest language-provided sequence first
        let remaining = &source[byte_pos..];
        let mut matched = false;

        // Try multi-char sequences in descending length order (pre-sorted by registry)
        for &multichar in token_reg.multichar_lexemes() {
            if remaining.starts_with(multichar) {
                // Found a multi-char match. Emit it and update position tracking.
                let lexeme = multichar.to_string();
                let span = Span::new(byte_pos, byte_pos + multichar.len());
                out.push(SpannedToken::new(Token::new(lexeme, span), line_no, start_col));

                // Update line/col for the matched sequence
                for byte in multichar.as_bytes() {
                    if *byte == b'\n' {
                        line_no += 1;
                        col_in_line = 1;
                    } else {
                        col_in_line += 1;
                    }
                }

                byte_pos += multichar.len();
                matched = true;
                break;
            }
        }

        if matched {
            continue;
        }

        // No multi-char match: emit single byte as token
        // Kernel does not reject any byte - even whitespace, control chars, etc.
        // Languages interpret all bytes according to their conventions.
        let byte = bytes[byte_pos];
        let lexeme = (byte as char).to_string();
        let span = Span::new(byte_pos, byte_pos + 1);
        out.push(SpannedToken::new(Token::new(lexeme, span), line_no, start_col));

        // Update line/col for the single byte
        if byte == b'\n' {
            line_no += 1;
            col_in_line = 1;
        } else {
            col_in_line += 1;
        }

        byte_pos += 1;
    }

    Ok(out)
}
