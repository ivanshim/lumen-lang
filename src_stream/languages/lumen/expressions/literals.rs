// Number, boolean, and string literals

use crate::src_stream::kernel::ast::ExprNode;
use crate::src_stream::kernel::parser::Parser;
use crate::src_stream::kernel::registry::{ExprPrefix, LumenResult, Registry};
use crate::src_stream::kernel::runtime::{Env, Value};
use crate::src_stream::src_lumen::values::{LumenNumber, LumenBool, LumenString};

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

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
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
        let lex = &parser.peek().lexeme;
        lex == "true" || lex == "false"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        let lexeme = parser.advance().lexeme;
        let value = lexeme == "true";
        Ok(Box::new(BoolLiteral { value }))
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
        Ok(Box::new(LumenString::new(content.to_string())))
    }
}

pub struct StringLiteralPrefix;

impl ExprPrefix for StringLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme starts with a double quote
        parser.peek().lexeme == "\""
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        // Consume opening quote
        let mut value = parser.advance().lexeme;

        // Since the kernel lexer is agnostic, it emits each character separately.
        // Assemble the full string by consuming characters until closing quote.
        loop {
            let ch = parser.peek().lexeme.clone();

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

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register handlers
    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
    reg.register_prefix(Box::new(StringLiteralPrefix));
}
