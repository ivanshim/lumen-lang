use crate::languages::lumen::prelude::*;
// Logical operators: and / or / not

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::values::{LumenBool, as_bool};

#[derive(Debug)]
struct LogicExpr {
    left: Box<dyn ExprNode>,
    op: String,
    right: Box<dyn ExprNode>,
}

impl ExprNode for LogicExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // Implement short-circuit evaluation
        let l = self.left.eval(env)?;
        let left_bool = as_bool(l.as_ref())?;

        match self.op.as_str() {
            "and" => {
                // Short-circuit: if left is false, don't evaluate right
                if !left_bool.value {
                    return Ok(Box::new(LumenBool::new(false)));
                }
                let r = self.right.eval(env)?;
                let right_bool = as_bool(r.as_ref())?;
                Ok(Box::new(LumenBool::new(right_bool.value)))
            }
            "or" => {
                // Short-circuit: if left is true, don't evaluate right
                if left_bool.value {
                    return Ok(Box::new(LumenBool::new(true)));
                }
                let r = self.right.eval(env)?;
                let right_bool = as_bool(r.as_ref())?;
                Ok(Box::new(LumenBool::new(right_bool.value)))
            }
            _ => Err(format!("Invalid logical operator: {}", self.op)),
        }
    }
}

pub struct LogicInfix {
    op: String,
}

impl LogicInfix {
    pub fn new(op: &str) -> Self {
        Self { op: op.to_string() }
    }
}

impl ExprInfix for LogicInfix {
    fn matches(&self, parser: &Parser) -> bool {
        let lex = &parser.peek().lexeme;

        // Case 1: "and"/"or" are registered as keyword tokens (single token)
        if lex == &self.op {
            return true;
        }

        // Case 2: "and"/"or" are split into characters (for backward compatibility with non-registered keywords)
        // Quick check: first character must match
        if self.op.chars().next() != lex.chars().next() {
            return false;
        }

        let mut i = parser.i;
        let mut collected = String::new();

        for expected_ch in self.op.chars() {
            if i >= parser.toks.len() {
                return false;
            }
            let actual = &parser.toks[i].tok.lexeme;
            if actual.len() == 1 && actual.chars().next() == Some(expected_ch) {
                collected.push(expected_ch);
                i += 1;
            } else {
                return false;
            }
        }

        // Make sure next character doesn't extend the operator
        if i < parser.toks.len() {
            let next = &parser.toks[i].tok.lexeme;
            if next.len() == 1 {
                let next_ch = next.chars().next().unwrap();
                if next_ch.is_ascii_alphanumeric() || next_ch == '_' {
                    return false;
                }
            }
        }

        collected == self.op
    }

    fn precedence(&self) -> Precedence {
        Precedence::Logic
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn ExprNode>> {
        // Consume the operator - either as a single token or as multiple characters
        if parser.peek().lexeme == self.op {
            // Single token operator (registered as keyword)
            parser.advance();
        } else {
            // Multi-character operator (individual character tokens)
            for _ in self.op.chars() {
                parser.advance();
            }
        }
        parser.skip_tokens();
        let right = parser.parse_expr_prec(registry, self.precedence() + 1)?;
        Ok(Box::new(LogicExpr { left, op: self.op.clone(), right }))
    }
}

// Unary NOT

#[derive(Debug)]
struct NotExpr {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for NotExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let val = self.expr.eval(env)?;
        let b = as_bool(val.as_ref())?;
        Ok(Box::new(LumenBool::new(!b.value)))
    }
}

pub struct NotPrefix;

impl ExprPrefix for NotPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        let lex = &parser.peek().lexeme;

        // Case 1: "not" is registered as a keyword token (single token)
        if lex == "not" {
            return true;
        }

        // Case 2: "not" is split into characters (for backward compatibility with non-registered keywords)
        // Quick check: first character must be 'n'
        if lex != "n" {
            return false;
        }

        // Collect "n", "o", "t" from character tokens
        let mut i = parser.i;
        let mut collected = String::new();

        for expected_ch in "not".chars() {
            if i >= parser.toks.len() {
                return false;
            }
            let actual = &parser.toks[i].tok.lexeme;
            if actual.len() == 1 && actual.chars().next() == Some(expected_ch) {
                collected.push(expected_ch);
                i += 1;
            } else {
                return false;
            }
        }

        // Make sure the next character doesn't extend it (like "notion")
        if i < parser.toks.len() {
            let next = &parser.toks[i].tok.lexeme;
            if next.len() == 1 {
                let next_ch = next.chars().next().unwrap();
                if next_ch.is_ascii_alphanumeric() || next_ch == '_' {
                    return false;
                }
            }
        }

        collected == "not"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        // Consume "not" - either as a single token or as multiple characters
        if parser.peek().lexeme == "not" {
            // Single token operator (registered as keyword)
            parser.advance();
        } else {
            // Multi-character operator (individual character tokens)
            for _ in "not".chars() {
                parser.advance();
            }
        }
        parser.skip_tokens();
        let expr = parser.parse_expr_prec(registry, Precedence::Unary)?;
        Ok(Box::new(NotExpr { expr }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["and", "or", "not"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register handlers
    reg.register_infix(Box::new(LogicInfix::new("and")));
    reg.register_infix(Box::new(LogicInfix::new("or")));
    reg.register_prefix(Box::new(NotPrefix));
}
