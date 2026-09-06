// Stage 1: Ingest — source text → tokens.
//
// Tokens are meaningful units: words, numbers, strings (already unescaped),
// operators, line ends and leading indentation. What counts as a comment,
// a string delimiter, an escape, a prologue, a variable prefix or an
// operator comes from the definition; the scanning algorithms are the
// kernel's.

use crate::schema::LanguageSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Word,
    Number,
    Str,
    Op,
    Newline,
    /// Leading whitespace of a non-blank line; `width` holds its size.
    Indent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    pub text: String,
    pub width: usize,
    pub line: usize,
    pub col: usize,
}

impl Token {
    fn new(kind: Kind, text: String, line: usize, col: usize) -> Self {
        Token { kind, text, width: 0, line, col }
    }

    pub fn is(&self, kind: Kind, text: &str) -> bool {
        self.kind == kind && self.text == text
    }
}

/// Drop the prologue (e.g. `<?php`) if the file opens with it.
fn strip_prologue<'a>(source: &'a str, schema: &LanguageSchema) -> &'a str {
    match &schema.lexical.prologue {
        Some(prologue) => {
            let lead = source.len() - source.trim_start().len();
            if source[lead..].starts_with(prologue.as_str()) {
                // Keep the leading whitespace so line numbers stay stable.
                let (head, _) = source.split_at(lead);
                let rest = &source[lead + prologue.len()..];
                // head is whitespace only; returning rest alone is exact if head has no newline.
                if head.contains('\n') {
                    return source;
                }
                return rest;
            }
            source
        }
        None => source,
    }
}

/// Remove comments outside strings, keeping newlines so line numbers stay
/// stable. Line comments run to the end of the line; block comments run
/// from the opening to the closing delimiter.
fn strip_comments(source: &str, schema: &LanguageSchema) -> String {
    let lexical = &schema.lexical;
    if lexical.comment_lines.is_empty() && lexical.comment_blocks.is_empty() {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    while let Some(ch) = rest.chars().next() {
        if let Some(q) = in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                in_string = None;
            }
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if lexical.quotes.contains(&ch) {
            in_string = Some(ch);
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if let Some((open, close)) = lexical.comment_blocks.iter().find(|(open, _)| rest.starts_with(open.as_str())) {
            let body = &rest[open.len()..];
            let end = body.find(close.as_str()).map(|p| p + close.len()).unwrap_or(body.len());
            out.extend(body[..end].chars().filter(|&c| c == '\n'));
            rest = &body[end..];
            continue;
        }
        if let Some(marker) = lexical.comment_lines.iter().find(|m| rest.starts_with(m.as_str())) {
            let _ = marker;
            rest = match rest.find('\n') {
                Some(nl) => &rest[nl..],
                None => "",
            };
            continue;
        }
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// Identifier characters: underscore always; letters (and, for continuation,
/// digits) from either ASCII or all of Unicode, as the definition chooses.
fn is_word_start(c: char, unicode: bool) -> bool {
    match (c, unicode) {
        ('_', _) => true,
        (c, true) => c.is_alphabetic(),
        (c, false) => c.is_ascii_alphabetic(),
    }
}

fn is_word_char(c: char, unicode: bool) -> bool {
    match (c, unicode) {
        ('_', _) => true,
        (c, true) => c.is_alphanumeric(),
        (c, false) => c.is_ascii_alphanumeric(),
    }
}

pub fn lex(source: &str, schema: &LanguageSchema) -> Result<Vec<Token>, String> {
    let source = strip_prologue(source, schema);
    let source = strip_comments(source, schema);
    let lexical = &schema.lexical;
    let operators = schema.operators_longest_first();
    let number = &lexical.number;
    let unicode = lexical.identifier_unicode;
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let mut col = 1;
    let mut at_line_start = true;

    while i < chars.len() {
        if at_line_start {
            // Measure indentation; blank lines produce nothing at all.
            let mut width = 0;
            let mut j = i;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                width += if chars[j] == '\t' { schema.structure.indent_size } else { 1 };
                j += 1;
            }
            let line_end = chars[j..].iter().position(|&c| c == '\n').map(|p| j + p).unwrap_or(chars.len());
            let blank = chars[j..line_end].iter().all(|c| c.is_whitespace());
            if blank {
                i = if line_end < chars.len() { line_end + 1 } else { line_end };
                line += 1;
                col = 1;
                continue;
            }
            let mut tok = Token::new(Kind::Indent, String::new(), line, 1);
            tok.width = width;
            tokens.push(tok);
            col += j - i;
            i = j;
            at_line_start = false;
        }

        let c = chars[i];

        if c == '\n' {
            tokens.push(Token::new(Kind::Newline, "\n".to_string(), line, col));
            i += 1;
            line += 1;
            col = 1;
            at_line_start = true;
            continue;
        }

        if c == ' ' || c == '\t' || c == '\r' {
            i += 1;
            col += 1;
            continue;
        }

        let start_col = col;

        // Strings: delimiter from the definition, escapes from its table.
        if lexical.quotes.contains(&c) {
            let quote = c;
            let escapes = lexical.escapes.get(&quote);
            let mut text = String::new();
            i += 1;
            col += 1;
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '\\' && i + 1 < chars.len() {
                    let next = chars[i + 1];
                    match escapes.and_then(|table| table.get(&next)) {
                        Some(replacement) => text.push_str(replacement),
                        None => {
                            text.push('\\');
                            text.push(next);
                        }
                    }
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == quote {
                    closed = true;
                    i += 1;
                    col += 1;
                    break;
                }
                if ch == '\n' {
                    line += 1;
                    col = 0;
                }
                text.push(ch);
                i += 1;
                col += 1;
            }
            if !closed {
                return Err(format!("Unterminated {} string", quote));
            }
            tokens.push(Token::new(Kind::Str, text, line, start_col));
            continue;
        }

        // Numbers: digits, then a hexadecimal tail, a base-N tail or a
        // decimal fraction.
        if c.is_ascii_digit() {
            let mut text = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                text.push(chars[i]);
                i += 1;
            }
            let hex_tail = number.hex_prefix.as_ref().and_then(|prefix| {
                let mut p = prefix.chars();
                let (digit, letter) = (p.next()?, p.next()?);
                let follows = i + 1 < chars.len() && chars[i + 1].is_ascii_hexdigit();
                (text.len() == 1 && text.starts_with(digit) && chars[i] == letter && follows).then_some(letter)
            });
            let at_base_marker = number.base_marker.map_or(false, |m| i < chars.len() && chars[i] == m);
            if let Some(letter) = hex_tail {
                text.push(letter);
                i += 1;
                while i < chars.len() && chars[i].is_ascii_hexdigit() {
                    text.push(chars[i]);
                    i += 1;
                }
            } else if at_base_marker {
                text.push(chars[i]);
                i += 1;
                while i < chars.len() {
                    let ch = chars[i];
                    let next_is_digit = i + 1 < chars.len() && chars[i + 1].is_ascii_alphanumeric();
                    if ch.is_ascii_alphanumeric()
                        || (number.decimal_point == Some(ch) && next_is_digit)
                        || (number.exponent_marker == Some(ch) && next_is_digit)
                    {
                        text.push(ch);
                        i += 1;
                    } else {
                        break;
                    }
                }
            } else if let Some(point) = number.decimal_point {
                if i + 1 < chars.len() && chars[i] == point && chars[i + 1].is_ascii_digit() {
                    text.push(point);
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        text.push(chars[i]);
                        i += 1;
                    }
                }
            }
            col += text.len();
            tokens.push(Token::new(Kind::Number, text, line, start_col));
            continue;
        }

        // Words: identifiers and keywords alike; meaning is decided later.
        // A variable prefix (`$`) followed by a word start belongs to the word.
        let prefixed = lexical.variable_prefix == Some(c)
            && i + 1 < chars.len()
            && is_word_start(chars[i + 1], unicode);
        if is_word_start(c, unicode) || prefixed {
            let mut text = String::new();
            if prefixed {
                text.push(c);
                i += 1;
            }
            while i < chars.len() && is_word_char(chars[i], unicode) {
                text.push(chars[i]);
                i += 1;
            }
            col += text.chars().count();
            let lowered = text.to_lowercase();
            if lexical.keywords_case_insensitive && lexical.reserved_words.contains(&lowered) {
                text = lowered;
            } else if lexical.identifiers_case_insensitive {
                text = lowered;
            }
            tokens.push(Token::new(Kind::Word, text, line, start_col));
            continue;
        }

        // Operators and punctuation: longest match from the definition.
        let rest: String = chars[i..].iter().take(8).collect();
        match operators.iter().find(|op| rest.starts_with(*op)) {
            Some(op) => {
                let n = op.chars().count();
                tokens.push(Token::new(Kind::Op, op.to_string(), line, start_col));
                i += n;
                col += n;
            }
            None => return Err(format!("Unexpected character '{}' at {}:{}", c, line, col)),
        }
    }

    tokens.push(Token::new(Kind::Eof, "EOF".to_string(), line, col));
    Ok(tokens)
}
