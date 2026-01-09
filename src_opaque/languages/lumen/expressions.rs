// Lumen expression handlers for opaque kernel
// These provide semantic knowledge to evaluate Lumen expressions

use crate::kernel::ast::RuntimeValue;
use std::sync::Arc;

/// Range expression support (for `..` operator)
pub fn eval_range(start: &RuntimeValue, end: &RuntimeValue, inclusive: bool) -> Result<RuntimeValue, String> {
    let s = start.downcast_ref::<i64>()
        .ok_or("Range start must be integer")?;
    let e = end.downcast_ref::<i64>()
        .ok_or("Range end must be integer")?;

    Ok(Arc::new((*s, *e, inclusive)) as RuntimeValue)
}
