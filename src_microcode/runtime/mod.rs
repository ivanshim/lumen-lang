// Runtime execution environment
//
// The runtime provides scope management, variable storage, and extern function dispatch.
// It is language-agnostic and interprets execution based on kernel primitives.

pub mod env;
pub mod extern_system;

pub use env::Environment;
