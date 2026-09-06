use crate::language::prelude::*;
// Comparison: equal, not equal, less, greater, at most, at least. The
// lexemes come from the definition; the operations are named here.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::KernelResult as LumenResult;
use crate::kernel::runtime::{Env, Value};
use crate::language::registry::{ExprInfix, Precedence, Registry};
use crate::language::values::{as_number, as_rational, as_real, LumenBool, LumenRational};
use num_bigint::BigInt;

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
        // Every comparison is derived from two floor operations, equality
        // and less-than: a > b is b < a, a <= b is not b < a, a >= b is
        // not a < b.
        let result = match self.op {
            Cmp::Eq => equal(&l, &r),
            Cmp::Ne => !equal(&l, &r),
            Cmp::Lt => less(&l, &r)?,
            Cmp::Gt => less(&r, &l)?,
            Cmp::Le => !less(&r, &l)?,
            Cmp::Ge => !less(&l, &r)?,
        };
        Ok(Box::new(LumenBool::new(result)))
    }
}

/// A number of any kind as an exact fraction, or None for a non-number.
fn fraction(v: &Value) -> Option<LumenRational> {
    if let Ok(real) = as_real(v.as_ref()) {
        return Some(LumenRational::new(real.numerator.clone(), real.denominator.clone()));
    }
    if let Ok(rat) = as_rational(v.as_ref()) {
        return Some(rat.clone());
    }
    if let Ok(num) = as_number(v.as_ref()) {
        return Some(LumenRational::new(num.value.clone(), BigInt::from(1)));
    }
    None
}

/// Equality: numbers by exact value whatever their kind; everything else
/// by its own notion, and values of different kinds are never equal.
fn equal(l: &Value, r: &Value) -> bool {
    if let (Some(a), Some(b)) = (fraction(l), fraction(r)) {
        return &a.numerator * &b.denominator == &b.numerator * &a.denominator;
    }
    l.eq_value(r.as_ref()).unwrap_or(false)
}

/// Less-than on numbers of any kind, by cross-multiplication.
fn less(l: &Value, r: &Value) -> LumenResult<bool> {
    match (fraction(l), fraction(r)) {
        (Some(a), Some(b)) => Ok(&a.numerator * &b.denominator < &b.numerator * &a.denominator),
        _ => Err("Cannot apply operators other than == and != to these types".into()),
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
