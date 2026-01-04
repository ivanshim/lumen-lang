// src/src_mini_python/mod.rs
// Mini-Python language module
// Python-like language implementation with indentation-based blocks

pub mod values;
mod numeric;
pub mod expressions;
pub mod statements;
pub mod structure;
pub mod registry;
pub mod prelude;

pub mod src_mini_python;

pub use src_mini_python::register_all;
