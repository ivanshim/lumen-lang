// Language schemas: declarative syntax definitions
//
// Each language is defined entirely as data in its schema.
// The kernel interprets code according to the schema - no language-specific logic in kernel.

pub mod lumen;
pub mod mini_rust;
pub mod mini_python;

pub use lumen::schema as lumen_schema;
pub use mini_rust::schema as mini_rust_schema;
pub use mini_python::schema as mini_python_schema;
