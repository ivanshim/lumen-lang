// src/src_lumen/mod.rs
// Lumen language module
// Complete language definition for Lumen

pub mod expressions;
pub mod statements;
pub mod structure;

// The dispatcher module
pub mod dispatcher {
    include!("src_lumen.rs");
}
