use crate::languages::lumen::prelude::*;
// Number, boolean, string, and none literals

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::values::{LumenNumber, LumenBool, LumenString, LumenNone};

#[derive(Debug)]
pub struct NumberLiteral {
    pub value: String,
}

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(LumenNumber::new(self.value.clone())))
    }
}

pub struct NumberLiteralPrefix;

impl ExprPrefix for NumberLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme starts with a digit
        parser.peek().lexeme.chars().next().map_or(false, |c| c.is_ascii_digit())
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        // Consume the first digit
        let mut value = parser.advance().lexeme;

        // Since the kernel lexer is fully agnostic, it emits each digit as a separate token.
        // We need to consume consecutive digit tokens to build the full number.
        loop {
            // Check if next token is a digit
            if parser.peek().lexeme.len() == 1 {
                let ch = parser.peek().lexeme.as_bytes()[0];
                if ch.is_ascii_digit() || ch == b'.' {
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
        // Check if the next characters form "true" or "false" (which are not registered as tokens)
        for keyword in &["true", "false"] {
            let mut i = parser.i;
            let mut collected = String::new();

            // Collect characters to check if they form our keyword
            for expected_ch in keyword.chars() {
                if i >= parser.toks.len() {
                    break;
                }
                let actual = &parser.toks[i].tok.lexeme;
                if actual.len() == 1 && actual.chars().next() == Some(expected_ch) {
                    collected.push(expected_ch);
                    i += 1;
                } else {
                    break;
                }
            }

            // Make sure we collected the full keyword
            if collected == *keyword {
                // Make sure next character doesn't extend the keyword
                if i < parser.toks.len() {
                    let next = &parser.toks[i].tok.lexeme;
                    if next.len() == 1 {
                        let next_ch = next.chars().next().unwrap();
                        if next_ch.is_ascii_alphanumeric() || next_ch == '_' {
                            continue; // This keyword is extended, try next one
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        // Determine which keyword we're parsing
        let keywords = ["true", "false"];
        let mut collected = String::new();
        let mut matched_keyword = "";

        for keyword in &keywords {
            let mut i = parser.i;
            collected.clear();

            // Collect characters to check if they form our keyword
            for expected_ch in keyword.chars() {
                if i >= parser.toks.len() {
                    break;
                }
                let actual = &parser.toks[i].tok.lexeme;
                if actual.len() == 1 && actual.chars().next() == Some(expected_ch) {
                    collected.push(expected_ch);
                    i += 1;
                } else {
                    break;
                }
            }

            // Check if we matched the full keyword
            if collected == *keyword {
                // Make sure next character doesn't extend the keyword
                if i < parser.toks.len() {
                    let next = &parser.toks[i].tok.lexeme;
                    if next.len() == 1 {
                        let next_ch = next.chars().next().unwrap();
                        if next_ch.is_ascii_alphanumeric() || next_ch == '_' {
                            continue; // This keyword is extended, try next one
                        }
                    }
                }
                matched_keyword = keyword;
                break;
            }
        }

        // Consume the matched keyword characters
        for _ in matched_keyword.chars() {
            parser.advance();
        }

        // Check if it's actually a boolean literal
        if matched_keyword == "true" {
            Ok(Box::new(BoolLiteral { value: true }))
        } else if matched_keyword == "false" {
            Ok(Box::new(BoolLiteral { value: false }))
        } else {
            // Not a boolean literal, this is an error
            Err(format!("Expected 'true' or 'false', got '{}'", matched_keyword))
        }
    }
}

// String literals

#[derive(Debug)]
struct StringLiteral {
    value: String,
}

impl ExprNode for StringLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        // Remove quotes from the tokenized string: "hello" -> hello
        let content = &self.value[1..self.value.len() - 1];
        // Process escape sequences
        let unescaped = content
            .replace("\\\"", "\"")  // Escaped quote
            .replace("\\\\", "\\")  // Escaped backslash
            .replace("\\n", "\n")   // Newline
            .replace("\\t", "\t");  // Tab
        Ok(Box::new(LumenString::new(unescaped)))
    }
}

pub struct StringLiteralPrefix;

impl ExprPrefix for StringLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme starts with a double quote
        parser.peek().lexeme == "\""
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        // Consume opening quote
        let mut value = parser.advance().lexeme;

        // Since the kernel lexer is agnostic, it emits each character separately.
        // Assemble the full string by consuming characters until closing quote (unescaped).
        loop {
            let ch = parser.peek().lexeme.clone();

            // Check for backslash (escape character)
            if ch == "\\" {
                value.push_str(&parser.advance().lexeme);
                // Consume the next character as escaped
                if parser.i < parser.toks.len() {
                    value.push_str(&parser.advance().lexeme);
                }
                continue;
            }

            // Check for closing quote
            if ch == "\"" {
                value.push_str(&parser.advance().lexeme);
                break;
            }

            // Add character to string (including whitespace, newlines, etc.)
            value.push_str(&parser.advance().lexeme);

            // Protect against unterminated strings
            if parser.i >= parser.toks.len() {
                return Err("Unterminated string literal".into());
            }
        }

        Ok(Box::new(StringLiteral { value }))
    }
}

// None literal

#[derive(Debug)]
struct NoneLiteral;

impl ExprNode for NoneLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(LumenNone))
    }
}

pub struct NoneLiteralPrefix;

impl ExprPrefix for NoneLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "none"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume 'none'
        Ok(Box::new(NoneLiteral))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["true", "false", "none", "\""])
        .with_char_classes(vec!["digit", "quote"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register handlers
    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
    reg.register_prefix(Box::new(NoneLiteralPrefix));
    reg.register_prefix(Box::new(StringLiteralPrefix));
}
