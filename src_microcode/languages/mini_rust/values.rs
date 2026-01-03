// Mini-Rust language-specific values
//
// Values are defined at the kernel level (kernel/eval.rs).
// This module is reserved for any Mini-Rust-specific value handling if needed.
//
// Mini-Rust supports the following value types through the kernel:
// - Number (f64): Used for all numeric types (i32, i64, f32, f64, etc. are mapped to this)
// - String: UTF-8 text
// - Boolean: true/false
// - Null: Represents void/None

// Currently, Mini-Rust uses the kernel's Value type directly.
// Language-specific value types could be added here if needed for:
// - Type checking and validation
// - Custom display formatting
// - Language-specific operations
