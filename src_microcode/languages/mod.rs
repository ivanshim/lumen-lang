// Language-specific declarative schemas
//
// Each language module provides ONLY data: no parsing logic, no semantic assumptions.
// The kernel interprets these tables to execute the language.
// Language modules must be purely declarative.

pub mod lumen;

pub use lumen::schema as lumen_schema;
