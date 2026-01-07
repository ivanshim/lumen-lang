// Language schemas: declarative syntax definitions
//
// Each language is defined entirely as data in its schema.
// The kernel interprets code according to the schema - no language-specific logic in kernel.

pub mod lumen;
pub mod rust;
pub mod python;

pub use lumen::schema as lumen_schema;
pub use rust::schema as rust_schema;
pub use python::schema as python_schema;
