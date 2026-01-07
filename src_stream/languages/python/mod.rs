// src/src_python/mod.rs
// Python language module
// Python-like language implementation with indentation-based blocks

pub mod values;
mod numeric;
pub mod expressions;
pub mod statements;
pub mod structure;
pub mod registry;
pub mod prelude;

pub mod src_python;

pub use src_python::register_all;
