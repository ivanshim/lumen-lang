use crate::languages::lumen::prelude::*;
// Comparison: equal, not equal, less, greater, at most, at least. The
// lexemes come from the definition; the operations are named here.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::KernelResult as LumenResult;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::registry::{ExprInfix, Precedence, Registry};
use crate::languages::lumen::numeric;
use crate::languages::lumen::values::{as_number, as_string, as_rational, as_real, LumenBool, LumenRational};

/// The comparison operations, independent of their spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug)]
struct ComparisonExpr {
    left: Box<dyn ExprNode>,
    op: Cmp,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ComparisonExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        // Check if either operand is Real and convert to Rational-like for comparison
        let (l_rat_opt, r_rat_opt) = (
            as_real(l.as_ref())
                .ok()
                .map(|real| LumenRational::new(real.numerator.clone(), real.denominator.clone()))
                .or_else(|| as_rational(l.as_ref()).ok().cloned()),
            as_real(r.as_ref())
                .ok()
                .map(|real| LumenRational::new(real.numerator.clone(), real.denominator.clone()))
                .or_else(|| as_rational(r.as_ref()).ok().cloned()),
        );

        // Try rational comparison first (handles rational-to-rational, real-to-real, real-to-rational)
        if let (Some(left_rat), Some(right_rat)) = (l_rat_opt.as_ref(), r_rat_opt.as_ref()) {
            let result = match self.op {
                Cmp::Eq => (left_rat as &dyn crate::kernel::runtime::RuntimeValue).eq_value(right_rat as &dyn crate::kernel::runtime::RuntimeValue).unwrap_or(false),
                Cmp::Ne => !(left_rat as &dyn crate::kernel::runtime::RuntimeValue).eq_value(right_rat as &dyn crate::kernel::runtime::RuntimeValue).unwrap_or(false),
                Cmp::Lt => {
                    // a/b < c/d ⟺ ad < bc (exact cross-multiplication)
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross < right_cross
                }
                Cmp::Gt => {
                    // a/b > c/d ⟺ ad > bc
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross > right_cross
                }
                Cmp::Le => {
                    // a/b <= c/d ⟺ ad <= bc
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross <= right_cross
                }
                Cmp::Ge => {
                    // a/b >= c/d ⟺ ad >= bc
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross >= right_cross
                }
            };
            return Ok(Box::new(LumenBool::new(result)));
        }

        // Try rational/real vs integer (convert integer to rational first)
        let left_rat_maybe = l_rat_opt.clone();
        if let (Some(left_rat), Ok(right_num)) = (left_rat_maybe, as_number(r.as_ref())) {
            let right_rat = LumenRational::new(right_num.value.clone(), num_bigint::BigInt::from(1));
            let result = match self.op {
                Cmp::Eq => (&left_rat as &dyn crate::kernel::runtime::RuntimeValue).eq_value(&right_rat as &dyn crate::kernel::runtime::RuntimeValue).unwrap_or(false),
                Cmp::Ne => !(&left_rat as &dyn crate::kernel::runtime::RuntimeValue).eq_value(&right_rat as &dyn crate::kernel::runtime::RuntimeValue).unwrap_or(false),
                Cmp::Lt => {
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross < right_cross
                }
                Cmp::Gt => {
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross > right_cross
                }
                Cmp::Le => {
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross <= right_cross
                }
                Cmp::Ge => {
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross >= right_cross
                }
            };
            return Ok(Box::new(LumenBool::new(result)));
        }

        // Try integer vs rational/real (convert integer to rational first)
        let right_rat_maybe = r_rat_opt.clone();
        if let (Ok(left_num), Some(right_rat)) = (as_number(l.as_ref()), right_rat_maybe) {
            let left_rat = LumenRational::new(left_num.value.clone(), num_bigint::BigInt::from(1));
            let result = match self.op {
                Cmp::Eq => (&left_rat as &dyn crate::kernel::runtime::RuntimeValue).eq_value(&right_rat as &dyn crate::kernel::runtime::RuntimeValue).unwrap_or(false),
                Cmp::Ne => !(&left_rat as &dyn crate::kernel::runtime::RuntimeValue).eq_value(&right_rat as &dyn crate::kernel::runtime::RuntimeValue).unwrap_or(false),
                Cmp::Lt => {
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross < right_cross
                }
                Cmp::Gt => {
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross > right_cross
                }
                Cmp::Le => {
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross <= right_cross
                }
                Cmp::Ge => {
                    let left_cross = &left_rat.numerator * &right_rat.denominator;
                    let right_cross = &right_rat.numerator * &left_rat.denominator;
                    left_cross >= right_cross
                }
            };
            return Ok(Box::new(LumenBool::new(result)));
        }

        // Try numeric (integer-only) comparison
        if let (Ok(left_num), Ok(right_num)) = (as_number(l.as_ref()), as_number(r.as_ref())) {
            let result = match self.op {
                Cmp::Eq => left_num.value == right_num.value,
                Cmp::Ne => left_num.value != right_num.value,
                Cmp::Lt => numeric::compare_lt(&left_num.value, &right_num.value)?,
                Cmp::Gt => numeric::compare_gt(&left_num.value, &right_num.value)?,
                Cmp::Le => numeric::compare_le(&left_num.value, &right_num.value)?,
                Cmp::Ge => numeric::compare_ge(&left_num.value, &right_num.value)?,
            };
            return Ok(Box::new(LumenBool::new(result)));
        }

        // Try string comparison
        if let (Ok(left_str), Ok(right_str)) = (as_string(l.as_ref()), as_string(r.as_ref())) {
            let result = match self.op {
                Cmp::Eq => left_str.value == right_str.value,
                Cmp::Ne => left_str.value != right_str.value,
                _ => return Err("String comparison only supports == and !=".into()),
            };
            return Ok(Box::new(LumenBool::new(result)));
        }

        // Handle equality comparisons for remaining types
        match self.op {
            Cmp::Eq => {
                // Try the built-in eq_value for same-type comparisons
                // If that fails, different types are not equal
                let result = l.eq_value(r.as_ref()).unwrap_or(false);
                Ok(Box::new(LumenBool::new(result)))
            }
            Cmp::Ne => {
                // Try the built-in eq_value for same-type comparisons
                // If that fails, different types are not equal (so != is true)
                let result = l.eq_value(r.as_ref()).unwrap_or(false);
                Ok(Box::new(LumenBool::new(!result)))
            }
            _ => Err("Cannot apply operators other than == and != to these types".into()),
        }
    }
}

/// One spelling of one comparison, at the tier the definition gives it.
pub struct ComparisonInfix {
    lexeme: String,
    op: Cmp,
    prec: Precedence,
}

impl ExprInfix for ComparisonInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.lexeme
    }

    fn precedence(&self) -> Precedence {
        self.prec
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        parser.skip_tokens();
        let right = parser.parse_expr_prec(registry, self.prec.right_operand(&self.lexeme))?;
        Ok(Box::new(ComparisonExpr { left, op: self.op, right }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    let labels = [
        ("op.eq", Cmp::Eq),
        ("op.ne", Cmp::Ne),
        ("op.lt", Cmp::Lt),
        ("op.gt", Cmp::Gt),
        ("op.le", Cmp::Le),
        ("op.ge", Cmp::Ge),
    ];
    for (label, op) in labels {
        for lexeme in def().list(label) {
            let prec = Precedence::binary(lexeme);
            reg.register_infix(Box::new(ComparisonInfix { lexeme: lexeme.clone(), op, prec }));
        }
    }
}
