// src/src_lumen/mod.rs
// Lumen language module
// Complete language definition for Lumen

pub mod registry;
pub mod prelude;
pub mod patterns;
pub mod values;
mod numeric;
pub mod expressions;
pub mod statements;
pub mod structure;
pub mod extern_system;

// The dispatcher module
pub mod dispatcher {
    include!("src_lumen.rs");
}
