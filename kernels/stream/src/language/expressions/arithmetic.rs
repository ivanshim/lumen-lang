use crate::language::prelude::*;
// src/expr/arithmetic.rs
//
// Arithmetic: add, subtract, multiply, divide, integer quotient, remainder,
// power, concatenation and negation. The lexemes that spell each operation
// come from the definition; the operations are named here.
// Supports integers, rationals, and real values (exact rational arithmetic + real precision)

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::KernelResult as LumenResult;
use crate::language::registry::{ExprInfix, ExprPrefix, Precedence, Registry};

/// The arithmetic operations, independent of their spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Quot,
    Rem,
    Pow,
    Concat,
}
use crate::kernel::runtime::{Env, Value};
use crate::language::values::{LumenNumber, LumenRational, LumenReal, as_number, as_rational, as_real};
use num_bigint::BigInt;
use num_traits::ToPrimitive;

#[derive(Debug)]
struct UnaryMinusExpr {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for UnaryMinusExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // Derived: -x is 0 - x, so a real keeps its precision and a rational stays exact
        let val = self.expr.eval(env)?;
        apply(Arith::Sub, Box::new(LumenNumber::new(BigInt::from(0))), val)
    }
}

pub struct UnaryMinusPrefix;

impl ExprPrefix for UnaryMinusPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("op.negate", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        let lexeme = parser.advance().lexeme; // the negation sign
        parser.skip_tokens();
        let expr = parser.parse_expr_prec(registry, Precedence::unary(&lexeme))?;
        Ok(Box::new(UnaryMinusExpr { expr }))
    }
}

#[derive(Debug)]
struct ArithmeticExpr {
    left: Box<dyn ExprNode>,
    op: Arith,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ArithmeticExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;
        apply(self.op, l, r)
    }
}

/// The operation on two evaluated operands.
pub fn apply(op: Arith, l: Value, r: Value) -> LumenResult<Value> {

    // Special handling for . operator: string concatenation with coercion
    if op == Arith::Concat {
        use crate::language::values::LumenString;

        // Coerce both operands to strings using str()
        let left_str = l.as_display_string();
        let right_str = r.as_display_string();
        let result = format!("{}{}", left_str, right_str);
        return Ok(Box::new(LumenString::new(result)));
    }

    // Special handling for + operator: with a string on either side it
    // concatenates the rendered operands, otherwise it adds
    if op == Arith::Add {
        use crate::language::values::{LumenString, as_string};

        if as_string(l.as_ref()).is_ok() || as_string(r.as_ref()).is_ok() {
            let result = format!("{}{}", l.as_display_string(), r.as_display_string());
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
    // A real operand makes the result real, as does division in a
    // language whose `/` yields a real (op.div.result).
    let result_is_real = left_is_real || right_is_real || (op == Arith::Div && def().div_real);

    // The remainder is derived: a - b * (a // b), so the identity
    // a == b * (a // b) + a % b holds for every kind of number.
    if op == Arith::Rem {
        let quotient = apply(Arith::Quot, l.clone_boxed(), r.clone_boxed())?;
        let product = apply(Arith::Mul, r.clone_boxed(), quotient)?;
        return apply(Arith::Sub, l, product);
    }

    // The integer quotient divides the exact values and truncates toward
    // zero. A real operand makes the result real.
    if op == Arith::Quot {
        let (left_num, left_den) = exact_parts(l.as_ref(), "Left operand must be a number")?;
        let (right_num, right_den) = exact_parts(r.as_ref(), "Right operand must be a number")?;
        let divisor = &right_num * &left_den;
        if divisor == BigInt::from(0) {
            return Err("Division by zero".into());
        }
        let result = (&left_num * &right_den) / divisor;

        // Determine result precision for real operations
        let result_precision = left_real_prec.or(right_real_prec).unwrap_or(15);

        // If result involves Real, return as LumenReal; otherwise as LumenNumber
        if result_is_real {
            return Ok(Box::new(LumenReal::new(result, BigInt::from(1), result_precision)));
        } else {
            return Ok(Box::new(LumenNumber::new(result)));
        }
    }

    // Exponentiation is derived: multiplication by squaring on the
    // exponent's integer part, so a real base keeps its precision.
    if op == Arith::Pow {
        let (exp_num, exp_den) = exact_parts(r.as_ref(), "Right operand must be a number")?;
        let mut exponent = (&exp_num / &exp_den).to_u64().ok_or_else(|| "Exponent too large".to_string())?;
        let mut base = l.clone_boxed();
        let mut acc: Value = if let Ok(real) = as_real(l.as_ref()) {
            Box::new(LumenReal::new(BigInt::from(1), BigInt::from(1), real.precision))
        } else if as_rational(l.as_ref()).is_ok() || as_number(l.as_ref()).is_ok() {
            Box::new(LumenNumber::new(BigInt::from(1)))
        } else {
            return Err("Left operand must be a number".into());
        };
        while exponent > 0 {
            if exponent & 1 == 1 {
                acc = apply(Arith::Mul, acc, base.clone_boxed())?;
            }
            exponent >>= 1;
            if exponent > 0 {
                base = apply(Arith::Mul, base.clone_boxed(), base)?;
            }
        }
        return Ok(acc);
    }

    // Try to extract left and right as numbers (integer, rational, or real)
    let (left_num, _left_is_rat) = if let Ok(real) = as_real(l.as_ref()) {
        (LumenRational::new(real.numerator.clone(), real.denominator.clone()), false)
    } else if let Ok(rat) = as_rational(l.as_ref()) {
        (rat.clone(), true)
    } else if let Ok(num) = as_number(l.as_ref()) {
        let rat = LumenRational::new(num.value.clone(), BigInt::from(1));
        (rat, false)
    } else {
        return Err("Left operand must be a number".into());
    };

    let (right_num, _right_is_rat) = if let Ok(real) = as_real(r.as_ref()) {
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

    let result = match op {
        Arith::Add => {
            // a/b + c/d = (ad + bc) / bd
            let num = left_num.numerator * &right_num.denominator + right_num.numerator * &left_num.denominator;
            let denom = left_num.denominator * right_num.denominator;
            LumenRational::new(num, denom)
        }
        Arith::Sub => {
            // a/b - c/d = (ad - bc) / bd
            let num = left_num.numerator * &right_num.denominator - right_num.numerator * &left_num.denominator;
            let denom = left_num.denominator * right_num.denominator;
            LumenRational::new(num, denom)
        }
        Arith::Mul => {
            // a/b * c/d = (ac) / (bd)
            let num = left_num.numerator * &right_num.numerator;
            let denom = left_num.denominator * right_num.denominator;
            LumenRational::new(num, denom)
        }
        Arith::Div => {
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

/// One spelling of one arithmetic operation, at the tier the definition gives it.
pub struct ArithmeticInfix {
    lexeme: String,
    op: Arith,
    prec: Precedence,
}

impl ExprInfix for ArithmeticInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.lexeme
    }

    fn precedence(&self) -> Precedence {
        self.prec
    }

    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        parser.skip_tokens();
        let right = parser.parse_expr_prec(registry, self.prec.right_operand(&self.lexeme))?;
        Ok(Box::new(ArithmeticExpr { left, op: self.op, right }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(UnaryMinusPrefix));
    let labels = [
        ("op.add", Arith::Add),
        ("op.sub", Arith::Sub),
        ("op.mul", Arith::Mul),
        ("op.div", Arith::Div),
        ("op.quot", Arith::Quot),
        ("op.rem", Arith::Rem),
        ("op.pow", Arith::Pow),
        ("op.concat", Arith::Concat),
    ];
    for (label, op) in labels {
        for lexeme in def().list(label) {
            let prec = Precedence::binary(lexeme);
            reg.register_infix(Box::new(ArithmeticInfix { lexeme: lexeme.clone(), op, prec }));
        }
    }
}

/// A number of any kind as an exact numerator and denominator.
fn exact_parts(value: &dyn crate::kernel::runtime::RuntimeValue, error: &str) -> LumenResult<(BigInt, BigInt)> {
    if let Ok(real) = as_real(value) {
        Ok((real.numerator.clone(), real.denominator.clone()))
    } else if let Ok(rat) = as_rational(value) {
        Ok((rat.numerator.clone(), rat.denominator.clone()))
    } else if let Ok(num) = as_number(value) {
        Ok((num.value.clone(), BigInt::from(1)))
    } else {
        Err(error.into())
    }
}
