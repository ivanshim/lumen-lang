use crate::languages::lumen::prelude::*;
// Number, boolean, string and null literals.
//
// Which words mean true, false and null, which characters delimit strings,
// which quotes are raw, which escape letters exist and how numbers are
// punctuated all come from the definition.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::values::{LumenNumber, LumenBool, LumenString, LumenNull, LumenReal};
use crate::languages::lumen::numeric::{self, NumberMarks};
use num_bigint::BigInt;

#[derive(Debug)]
pub struct NumberLiteral {
    pub value: String,
}

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        let (numerator, denominator) = numeric::parse_number_rational(&self.value)?;

        // If denominator is 1, it's an integer - use Number
        if denominator == BigInt::from(1) {
            Ok(Box::new(LumenNumber::new(numerator)))
        } else {
            // Float literal with decimal places - create Real value
            // Precision is determined by significant figures in the literal
            let precision = calculate_precision(&self.value);
            Ok(Box::new(LumenReal::new(numerator, denominator, precision)))
        }
    }
}

/// Calculate precision (significant figures) from a float literal string
/// E.g., "1.5" -> 2, "3.14" -> 3, "0.05" -> 1
fn calculate_precision(s: &str) -> usize {
    // Keep the digits only
    let digits: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect();

    // Count leading zeros to skip them
    let leading_zeros = digits.chars().take_while(|c| *c == '0').count();

    // Significant figures = total digits minus leading zeros
    let significant_count = digits.len().saturating_sub(leading_zeros);

    // Use at least 15 significant figures as default minimum
    std::cmp::max(significant_count.max(1), 15)
}

pub struct NumberLiteralPrefix;

impl ExprPrefix for NumberLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme starts with a digit
        parser.peek().lexeme.chars().next().map_or(false, |c| c.is_ascii_digit())
    }

    fn parse(&self, parser: &mut Parser, _registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        let marks = NumberMarks::from_definition();
        // Consume the first digit
        let mut value = parser.advance().lexeme;
        let mut in_base_n = false;

        // Since the kernel lexer is fully agnostic, it emits each digit as a separate token.
        // We need to consume consecutive digit tokens to build the full number.
        // For base-N literals: <base>@<digits>[.<fraction>][^<exponent>]
        loop {
            if parser.peek().lexeme.chars().count() == 1 {
                let ch = parser.peek().lexeme.chars().next().unwrap();
                let punctuation = Some(ch) == marks.point
                    || Some(ch) == marks.base
                    || (in_base_n && Some(ch) == marks.exponent);
                if Some(ch) == marks.base {
                    in_base_n = true;
                }
                if ch.is_ascii_digit() || punctuation {
                    value.push_str(&parser.advance().lexeme);
                    continue;
                }
                // Within a base-N literal the digits may be letters too.
                if in_base_n && ch.is_ascii_alphabetic() {
                    value.push_str(&parser.advance().lexeme);
                    continue;
                }
            }
            break;
        }

        Ok(Box::new(NumberLiteral { value }))
    }
}

// Boolean literals

#[derive(Debug)]
struct BoolLiteral {
    value: bool,
}

impl ExprNode for BoolLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(LumenBool::new(self.value)))
    }
}

pub struct BoolLiteralPrefix;

impl ExprPrefix for BoolLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        let lexeme = &parser.peek().lexeme;
        def().is("literal.true", lexeme) || def().is("literal.false", lexeme)
    }

    fn parse(&self, parser: &mut Parser, _registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        let word = parser.advance().lexeme;
        Ok(Box::new(BoolLiteral { value: def().is("literal.true", &word) }))
    }
}

// String literals

#[derive(Debug)]
struct StringLiteral {
    /// The characters between the quotes, escapes still in place.
    content: String,
    quote: char,
}

impl ExprNode for StringLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(LumenString::new(unescape(&self.content, self.quote))))
    }
}

/// Resolve escapes. Inside a raw quote only the quote itself and the
/// backslash may be escaped; inside any other quote the definition's escape
/// letters apply too. A backslash before an unlisted letter stays as written.
fn unescape(s: &str, quote: char) -> String {
    let d = def();
    let raw = d.chars("lexical.raw_quotes").contains(&quote);
    let letters = if raw { Vec::new() } else { d.chars("lexical.string_escapes") };
    let quotes = d.chars("lexical.string_quotes");
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        let Some(&next) = chars.peek() else {
            result.push(ch); // trailing backslash
            continue;
        };
        let replacement = if next == quote || next == '\\' {
            Some(next.to_string())
        } else if letters.contains(&next) {
            match next {
                'n' => Some("\n".to_string()),
                't' => Some("\t".to_string()),
                'r' => Some("\r".to_string()),
                '0' => Some("\0".to_string()),
                c if quotes.contains(&c) => Some(c.to_string()),
                _ => None,
            }
        } else {
            None
        };
        match replacement {
            Some(text) => {
                chars.next();
                result.push_str(&text);
            }
            None => result.push(ch), // backslash followed by other char - keep as-is
        }
    }
    result
}

pub struct StringLiteralPrefix;

impl ExprPrefix for StringLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        opening_quote(&parser.peek().lexeme).is_some()
    }

    fn parse(&self, parser: &mut Parser, _registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        let quote = opening_quote(&parser.peek().lexeme).expect("matches() saw a quote");
        parser.advance(); // opening quote
        let content = take_string_body(parser, quote)?;
        Ok(Box::new(StringLiteral { content, quote }))
    }
}

/// The quote character a one-character lexeme is, if it is one.
fn opening_quote(lexeme: &str) -> Option<char> {
    let mut chars = lexeme.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if def().chars("lexical.string_quotes").contains(&c) => Some(c),
        _ => None,
    }
}

/// Consume tokens up to and including the closing `quote`, returning the
/// characters between the quotes with escapes still in place. The kernel
/// lexer emits each character separately, so the body is reassembled here.
pub fn take_string_body(parser: &mut Parser, quote: char) -> LumenResult<String> {
    let mut content = String::new();
    loop {
        if parser.i >= parser.toks.len() {
            return Err("Unterminated string literal".into());
        }
        let lexeme = parser.advance().lexeme;
        if lexeme == "\\" {
            content.push_str(&lexeme);
            // The escaped character, whatever it is
            if parser.i < parser.toks.len() {
                content.push_str(&parser.advance().lexeme);
            }
            continue;
        }
        if lexeme.chars().count() == 1 && lexeme.starts_with(quote) {
            return Ok(content);
        }
        content.push_str(&lexeme);
    }
}

// Null literal

#[derive(Debug)]
struct NullLiteral;

impl ExprNode for NullLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(LumenNull))
    }
}

pub struct NullLiteralPrefix;

impl ExprPrefix for NullLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("literal.null", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, _registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume the null word
        Ok(Box::new(NullLiteral))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
    reg.register_prefix(Box::new(NullLiteralPrefix));
    reg.register_prefix(Box::new(StringLiteralPrefix));
}
