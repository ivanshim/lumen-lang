use crate::languages::rust_core::prelude::*;
// Number and boolean literals for mini-rust

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::rust_core::registry::{ExprPrefix, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::rust_core::values::{RustCoreNumber, RustCoreBool};

// --------------------
// Token definitions
// --------------------

pub const TRUE: &str = "true";
pub const FALSE: &str = "false";

#[derive(Debug)]
pub struct NumberLiteral {
    pub value: String,
}

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(RustCoreNumber::new(self.value.clone())))
    }
}

pub struct NumberLiteralPrefix;

impl ExprPrefix for NumberLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
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
pub struct BoolLiteral {
    value: bool,
}

impl ExprNode for BoolLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(RustCoreBool::new(self.value)))
    }
}

pub struct BoolLiteralPrefix;

impl ExprPrefix for BoolLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        (parser.peek().lexeme == "true" || parser.peek().lexeme == "false")
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        { let value = parser.advance().lexeme == "true"; Ok(Box::new(BoolLiteral { value })) }
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register tokens
    // Register handlers
    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
}
