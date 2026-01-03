// Runtime: external function dispatch
//
// The runtime handles extern/foreign function calls defined in language schemas.
// Environment and scoping is handled in kernel/env.rs

pub mod r#extern;

pub use r#extern::execute_extern;
