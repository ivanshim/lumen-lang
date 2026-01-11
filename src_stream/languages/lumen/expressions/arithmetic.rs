use crate::languages::lumen::prelude::*;
// src/expr/arithmetic.rs
//
// + - * / % ** and unary minus
// Supports both integers and rationals (exact rational arithmetic)

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::registry::LumenResult;
use crate::languages::lumen::registry::{ExprInfix, ExprPrefix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::numeric;
use crate::languages::lumen::values::{LumenNumber, LumenRational, as_number, as_rational};
use num_bigint::BigInt;

#[derive(Debug)]
struct UnaryMinusExpr {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for UnaryMinusExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let val = self.expr.eval(env)?;

        // Handle rational negation
        if let Ok(rat) = as_rational(val.as_ref()) {
            return Ok(Box::new(LumenRational::new(-rat.numerator.clone(), rat.denominator.clone())));
        }

        // Handle integer negation
        let num = as_number(val.as_ref())?;
        let result = numeric::negate(&num.value)?;
        Ok(Box::new(LumenNumber::new(result)))
    }
}

pub struct UnaryMinusPrefix;

impl ExprPrefix for UnaryMinusPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "-"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // '-'
        parser.skip_tokens();
        let expr = parser.parse_expr_prec(registry, Precedence::Unary)?;
        Ok(Box::new(UnaryMinusExpr { expr }))
    }
}

#[derive(Debug)]
struct ArithmeticExpr {
    left: Box<dyn ExprNode>,
    op: String,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ArithmeticExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        // Special handling for + operator: can be string concatenation or addition
        if self.op == "+" {
            use crate::languages::lumen::values::{LumenString, as_string};

            // Try to treat both as strings for concatenation
            if let (Ok(left_str), Ok(right_str)) = (as_string(l.as_ref()), as_string(r.as_ref())) {
                let result = format!("{}{}", left_str.value, right_str.value);
                return Ok(Box::new(LumenString::new(result)));
            }
        }

        // Fast path for modulo and exponentiation (integer-only operations)
        // This avoids expensive rational conversion and cloning for these operators
        if self.op == "%" || self.op == "**" {
            // Extract integers directly by reference, then clone only if needed
            let result = if let Ok(num) = as_number(l.as_ref()) {
                let left_ref = &num.value;
                if let Ok(num2) = as_number(r.as_ref()) {
                    let right_ref = &num2.value;
                    if self.op == "%" {
                        numeric::modulo(left_ref, right_ref)?
                    } else {
                        numeric::power(left_ref, right_ref)?
                    }
                } else if let Ok(rat) = as_rational(r.as_ref()) {
                    if self.op == "%" {
                        numeric::modulo(left_ref, &rat.numerator)?
                    } else {
                        numeric::power(left_ref, &rat.numerator)?
                    }
                } else {
                    return Err("Right operand must be a number".into());
                }
            } else if let Ok(rat) = as_rational(l.as_ref()) {
                let left_ref = &rat.numerator;
                if let Ok(num) = as_number(r.as_ref()) {
                    let right_ref = &num.value;
                    if self.op == "%" {
                        numeric::modulo(left_ref, right_ref)?
                    } else {
                        numeric::power(left_ref, right_ref)?
                    }
                } else if let Ok(rat2) = as_rational(r.as_ref()) {
                    if self.op == "%" {
                        numeric::modulo(left_ref, &rat2.numerator)?
                    } else {
                        numeric::power(left_ref, &rat2.numerator)?
                    }
                } else {
                    return Err("Right operand must be a number".into());
                }
            } else {
                return Err("Left operand must be a number".into());
            };
            return Ok(Box::new(LumenNumber::new(result)));
        }

        // Try to extract left and right as numbers (integer or rational)
        let (left_num, left_is_rat) = if let Ok(rat) = as_rational(l.as_ref()) {
            (rat.clone(), true)
        } else if let Ok(num) = as_number(l.as_ref()) {
            let rat = LumenRational::new(num.value.clone(), BigInt::from(1));
            (rat, false)
        } else {
            return Err("Left operand must be a number".into());
        };

        let (right_num, right_is_rat) = if let Ok(rat) = as_rational(r.as_ref()) {
            (rat.clone(), true)
        } else if let Ok(num) = as_number(r.as_ref()) {
            let rat = LumenRational::new(num.value.clone(), BigInt::from(1));
            (rat, false)
        } else {
            return Err("Right operand must be a number".into());
        };

        // Check if either operand is rational
        let result_is_rational = left_is_rat || right_is_rat;

        let result = match self.op.as_str() {
            "+" => {
                // a/b + c/d = (ad + bc) / bd
                let num = left_num.numerator * &right_num.denominator + right_num.numerator * &left_num.denominator;
                let denom = left_num.denominator * right_num.denominator;
                LumenRational::new(num, denom)
            }
            "-" => {
                // a/b - c/d = (ad - bc) / bd
                let num = left_num.numerator * &right_num.denominator - right_num.numerator * &left_num.denominator;
                let denom = left_num.denominator * right_num.denominator;
                LumenRational::new(num, denom)
            }
            "*" => {
                // a/b * c/d = (ac) / (bd)
                let num = left_num.numerator * &right_num.numerator;
                let denom = left_num.denominator * right_num.denominator;
                LumenRational::new(num, denom)
            }
            "/" => {
                // a/b ÷ c/d = (ad) / (bc)
                if right_num.numerator == BigInt::from(0) {
                    return Err("Division by zero".into());
                }
                let num = left_num.numerator * &right_num.denominator;
                let denom = left_num.denominator * right_num.numerator;
                LumenRational::new(num, denom)
            }
            _ => return Err("Invalid arithmetic operator".into()),
        };

        // If result is an integer (denominator = 1), return as LumenNumber
        if result.is_integer() {
            Ok(Box::new(LumenNumber::new(result.numerator)))
        } else {
            Ok(Box::new(result))
        }
    }
}

pub struct ArithmeticInfix {
    op: String,
    prec: Precedence,
}

impl ArithmeticInfix {
    pub fn new(op: &str, prec: Precedence) -> Self {
        Self { op: op.to_string(), prec }
    }
}

impl ExprInfix for ArithmeticInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.op
    }

    fn precedence(&self) -> Precedence {
        self.prec
    }

    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        parser.skip_tokens();
        let right = parser.parse_expr_prec(registry, self.precedence() + 1)?;
        Ok(Box::new(ArithmeticExpr { left, op: self.op.clone(), right }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["+", "-", "*", "/", "%", "**"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register handlers
    reg.register_prefix(Box::new(UnaryMinusPrefix));
    reg.register_infix(Box::new(ArithmeticInfix::new("+", Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new("-", Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new("*", Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new("/", Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new("%", Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new("**", Precedence::Power)));
}
