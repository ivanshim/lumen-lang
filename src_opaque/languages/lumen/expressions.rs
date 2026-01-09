// Lumen expression handlers for opaque kernel
// These provide semantic knowledge to evaluate Lumen expressions

use crate::kernel::ast::{RuntimeValue, OpaqueAnalysis};
use std::any::Any;

/// Lumen-specific expression analysis
#[derive(Debug, Clone)]
pub struct LumenExprAnalysis {
    pub expr_type: String,
    pub metadata: Box<dyn Any + Send + Sync>,
}

impl LumenExprAnalysis {
    pub fn new(expr_type: impl Into<String>) -> Self {
        LumenExprAnalysis {
            expr_type: expr_type.into(),
            metadata: Box::new(()),
        }
    }
}

/// Range expression support (for `..` operator)
pub fn eval_range(start: &RuntimeValue, end: &RuntimeValue, inclusive: bool) -> Result<RuntimeValue, String> {
    let s = start.downcast_ref::<i64>()
        .ok_or("Range start must be integer")?;
    let e = end.downcast_ref::<i64>()
        .ok_or("Range end must be integer")?;

    Ok(Box::new((*s, *e, inclusive)) as RuntimeValue)
}
