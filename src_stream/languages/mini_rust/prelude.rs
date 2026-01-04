// Mini-Rust prelude - convenient import for handler modules
// All expression, statement, and structure modules can use:
// use crate::languages::mini_rust::prelude::*;

pub use crate::kernel::ast::{ExprNode, StmtNode};
pub use crate::kernel::parser::Parser;
pub use crate::kernel::registry::{LumenResult, err_at};
pub use crate::languages::mini_rust::registry::{
    ExprPrefix, ExprInfix, StmtHandler, Registry, Precedence, parse_expr_with_prec,
};
