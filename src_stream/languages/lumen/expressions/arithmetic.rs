use crate::languages::lumen::prelude::*;
// src/expr/arithmetic.rs
//
// + - * / % // ** and unary minus
// Supports integers, rationals, and real values (exact rational arithmetic + real precision)

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::registry::LumenResult;
use crate::languages::lumen::registry::{ExprInfix, ExprPrefix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::numeric;
use crate::languages::lumen::values::{LumenNumber, LumenRational, LumenReal, as_number, as_rational, as_real};
use num_bigint::BigInt;
use num_traits::ToPrimitive;

#[derive(Debug)]
struct UnaryMinusExpr {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for UnaryMinusExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let val = self.expr.eval(env)?;

        // Handle real negation
        if let Ok(real) = as_real(val.as_ref()) {
            return Ok(Box::new(LumenReal::new(-real.numerator.clone(), real.denominator.clone(), real.precision)));
        }

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

        // Special handling for . operator: string concatenation with coercion
        if self.op == "." {
            use crate::languages::lumen::values::LumenString;

            // Coerce both operands to strings using str()
            let left_str = l.as_display_string();
            let right_str = r.as_display_string();
            let result = format!("{}{}", left_str, right_str);
            return Ok(Box::new(LumenString::new(result)));
        }

        // Special handling for + operator: can be string concatenation or addition
        if self.op == "+" {
            use crate::languages::lumen::values::{LumenString, as_string};

            // Try to treat both as strings for concatenation
            if let (Ok(left_str), Ok(right_str)) = (as_string(l.as_ref()), as_string(r.as_ref())) {
                let result = format!("{}{}", left_str.value, right_str.value);
                return Ok(Box::new(LumenString::new(result)));
            }
        }

        // Check if either operand is real (Real takes precedence)
        let (left_real_prec, left_is_real) = if let Ok(real) = as_real(l.as_ref()) {
            (Some(real.precision), true)
        } else {
            (None, false)
        };
        let (right_real_prec, right_is_real) = if let Ok(real) = as_real(r.as_ref()) {
            (Some(real.precision), true)
        } else {
            (None, false)
        };
        let result_is_real = left_is_real || right_is_real;

        // Fast path for modulo and integer quotient (integer-only operations)
        // For Real values, extract the integer part and perform the operation
        // This avoids expensive rational conversion and cloning for these operators
        if self.op == "%" || self.op == "//" {
            // Extract integers directly by reference, then clone only if needed
            let result = if let Ok(real) = as_real(l.as_ref()) {
                let left_int = &real.numerator / &real.denominator;
                if let Ok(real2) = as_real(r.as_ref()) {
                    let right_int = &real2.numerator / &real2.denominator;
                    if self.op == "//" {
                        if right_int == BigInt::from(0) {
                            return Err("Division by zero".into());
                        }
                        &left_int / &right_int
                    } else {
                        numeric::modulo(&left_int, &right_int)?
                    }
                } else if let Ok(num) = as_number(r.as_ref()) {
                    if self.op == "//" {
                        if num.value == BigInt::from(0) {
                            return Err("Division by zero".into());
                        }
                        &left_int / &num.value
                    } else {
                        numeric::modulo(&left_int, &num.value)?
                    }
                } else if let Ok(rat) = as_rational(r.as_ref()) {
                    if self.op == "//" {
                        if rat.numerator == BigInt::from(0) {
                            return Err("Division by zero".into());
                        }
                        &left_int / &rat.numerator
                    } else {
                        numeric::modulo(&left_int, &rat.numerator)?
                    }
                } else {
                    return Err("Right operand must be a number".into());
                }
            } else if let Ok(num) = as_number(l.as_ref()) {
                let left_ref = &num.value;
                if let Ok(num2) = as_number(r.as_ref()) {
                    let right_ref = &num2.value;
                    if self.op == "%" {
                        numeric::modulo(left_ref, right_ref)?
                    } else {
                        if right_ref == &BigInt::from(0) {
                            return Err("Division by zero".into());
                        }
                        left_ref / right_ref
                    }
                } else if let Ok(rat) = as_rational(r.as_ref()) {
                    if self.op == "%" {
                        numeric::modulo(left_ref, &rat.numerator)?
                    } else {
                        if rat.numerator == BigInt::from(0) {
                            return Err("Division by zero".into());
                        }
                        left_ref / &rat.numerator
                    }
                } else {
                    return Err("Right operand must be a number".into());
                }
            } else if let Ok(rat) = as_rational(l.as_ref()) {
                // For modulo/quotient with rationals, extract integer part first (numerator / denominator)
                let left_int = &rat.numerator / &rat.denominator;
                if let Ok(num) = as_number(r.as_ref()) {
                    let right_ref = &num.value;
                    if self.op == "%" {
                        numeric::modulo(&left_int, right_ref)?
                    } else {
                        if right_ref == &BigInt::from(0) {
                            return Err("Division by zero".into());
                        }
                        &left_int / right_ref
                    }
                } else if let Ok(rat2) = as_rational(r.as_ref()) {
                    // For modulo/quotient with two rationals, extract integer parts from both
                    let right_int = &rat2.numerator / &rat2.denominator;
                    if self.op == "%" {
                        numeric::modulo(&left_int, &right_int)?
                    } else {
                        if right_int == BigInt::from(0) {
                            return Err("Division by zero".into());
                        }
                        &left_int / &right_int
                    }
                } else {
                    return Err("Right operand must be a number".into());
                }
            } else {
                return Err("Left operand must be a number".into());
            };

            // Determine result precision for real operations
            let result_precision = left_real_prec.or(right_real_prec).unwrap_or(15);

            // If result involves Real, return as LumenReal; otherwise as LumenNumber
            if result_is_real {
                return Ok(Box::new(LumenReal::new(result, BigInt::from(1), result_precision)));
            } else {
                return Ok(Box::new(LumenNumber::new(result)));
            }
        }

        // Handle exponentiation separately (base can be any numeric type, exponent must be integer)
        if self.op == "**" {
            // Extract base as rational (supports integer, rational, and real)
            let base_num = if let Ok(real) = as_real(l.as_ref()) {
                LumenRational::new(real.numerator.clone(), real.denominator.clone())
            } else if let Ok(rat) = as_rational(l.as_ref()) {
                rat.clone()
            } else if let Ok(num) = as_number(l.as_ref()) {
                LumenRational::new(num.value.clone(), BigInt::from(1))
            } else {
                return Err("Left operand must be a number".into());
            };

            // Extract exponent as integer (truncate rational/real to integer)
            let exp_int = if let Ok(num) = as_number(r.as_ref()) {
                num.value.clone()
            } else if let Ok(rat) = as_rational(r.as_ref()) {
                &rat.numerator / &rat.denominator
            } else if let Ok(real) = as_real(r.as_ref()) {
                &real.numerator / &real.denominator
            } else {
                return Err("Right operand must be a number".into());
            };

            // Convert exponent to u32 for pow operation
            let exp_u32 = exp_int.to_u32()
                .ok_or_else(|| "Exponent too large".to_string())?;

            // Compute base^exp for rational: (a/b)^n = a^n / b^n
            let result_num = base_num.numerator.pow(exp_u32);
            let result_denom = base_num.denominator.pow(exp_u32);
            let result_rational = LumenRational::new(result_num, result_denom);

            // Determine result precision for real operations
            let result_precision = left_real_prec.or(right_real_prec).unwrap_or(15);

            // If result involves Real, return as LumenReal; otherwise check if rational or integer
            if result_is_real {
                return Ok(Box::new(LumenReal::new(result_rational.numerator, result_rational.denominator, result_precision)));
            } else if result_rational.denominator == BigInt::from(1) {
                // Result is an integer
                return Ok(Box::new(LumenNumber::new(result_rational.numerator)));
            } else {
                // Result is a rational
                return Ok(Box::new(result_rational));
            }
        }

        // Try to extract left and right as numbers (integer, rational, or real)
        let (left_num, left_is_rat) = if let Ok(real) = as_real(l.as_ref()) {
            (LumenRational::new(real.numerator.clone(), real.denominator.clone()), false)
        } else if let Ok(rat) = as_rational(l.as_ref()) {
            (rat.clone(), true)
        } else if let Ok(num) = as_number(l.as_ref()) {
            let rat = LumenRational::new(num.value.clone(), BigInt::from(1));
            (rat, false)
        } else {
            return Err("Left operand must be a number".into());
        };

        let (right_num, right_is_rat) = if let Ok(real) = as_real(r.as_ref()) {
            (LumenRational::new(real.numerator.clone(), real.denominator.clone()), false)
        } else if let Ok(rat) = as_rational(r.as_ref()) {
            (rat.clone(), true)
        } else if let Ok(num) = as_number(r.as_ref()) {
            let rat = LumenRational::new(num.value.clone(), BigInt::from(1));
            (rat, false)
        } else {
            return Err("Right operand must be a number".into());
        };

        // Determine result precision for real operations
        let result_precision = left_real_prec.or(right_real_prec).unwrap_or(15);
        // Check if either operand is rational (when not real)
        let result_is_rational = !result_is_real && (left_is_rat || right_is_rat);

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

        // If result involves Real, return as LumenReal
        if result_is_real {
            Ok(Box::new(LumenReal::new(result.numerator, result.denominator, result_precision)))
        }
        // If result is an integer (denominator = 1), return as LumenNumber
        else if result.is_integer() {
            Ok(Box::new(LumenNumber::new(result.numerator)))
        }
        // Otherwise return as LumenRational
        else {
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
        .with_literals(vec!["+", "-", "*", "/", "%", "//", "**", "."])
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
    reg.register_infix(Box::new(ArithmeticInfix::new("//", Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new("**", Precedence::Power)));
    reg.register_infix(Box::new(ArithmeticInfix::new(".", Precedence::Factor))); // String concatenation with coercion
}
