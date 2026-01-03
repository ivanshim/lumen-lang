// Lumen language schema (pure declarative)
//
// This module contains ONLY data describing the Lumen language.
// No parsing logic - the kernel interprets this schema.

pub mod schema;
pub mod values;

pub use schema::get_schema;
