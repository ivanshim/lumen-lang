// Language schemas: declarative syntax definitions
//
// Each language is defined entirely as data in its schema.
// The kernel interprets code according to the schema - no language-specific logic in kernel.

pub mod lumen;
pub mod rust_core;
pub mod python_core;

pub use lumen::schema as lumen_schema;
pub use rust_core::schema as rust_core_schema;
pub use python_core::schema as python_core_schema;
