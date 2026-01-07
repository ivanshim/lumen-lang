// src/src_python_core/mod.rs
// Python language module
// Python-like language implementation with indentation-based blocks

pub mod values;
mod numeric;
pub mod expressions;
pub mod statements;
pub mod structure;
pub mod registry;
pub mod prelude;

pub mod src_python_core;

pub use src_python_core::register_all;
