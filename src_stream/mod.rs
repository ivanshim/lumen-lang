// Stream kernel track (procedural model)
// 
// This module provides the original stream-based kernel with
// procedural parsing in language modules.
// Completely independent from src_microcode.

pub mod kernel;
pub mod languages;

pub use kernel::*;
pub use languages::{src_lumen, src_rust, src_python};
