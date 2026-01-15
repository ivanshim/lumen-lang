// src/kernel/runtime/value.rs
//
// Runtime Value abstraction - kernel level (ONTOLOGICALLY NEUTRAL)
//
// The kernel operates on opaque values via the RuntimeValue trait.
// The kernel does NOT know about concrete types (Number, Bool, String, etc).
// Languages define and implement their own value types.
//
// This abstraction allows the kernel to:
// - Store and pass values between scopes
// - Invoke language-defined operations on values
// But the kernel never interprets what a value represents.

use std::fmt;
use std::any::Any;

/// Trait for language-specific runtime values.
/// Languages implement this for their concrete value types.
/// The kernel treats all values opaquely via this trait.
pub trait RuntimeValue: Send + Sync {
    /// Return a clone of this value as a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn RuntimeValue>;

    /// Return a debug representation of this value.
    fn as_debug_string(&self) -> String;

    /// Return a display representation of this value.
    /// This is what gets printed by print statements.
    fn as_display_string(&self) -> String;

    /// Check equality with another value.
    /// Returns an error if the types cannot be compared.
    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String>;

    /// Support for type-safe downcasting in language code.
    /// Returns this value as a reference to the concrete type (for use with Any::downcast_ref).
    fn as_any(&self) -> &dyn Any;

    /// Support for mutable type-safe downcasting (for mutation).
    /// Returns a mutable reference for use with Any::downcast_mut.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl Clone for Box<dyn RuntimeValue> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

impl PartialEq for Box<dyn RuntimeValue> {
    fn eq(&self, other: &Self) -> bool {
        // Use the trait method for equality, ignore errors in PartialEq context
        self.eq_value(other.as_ref()).unwrap_or(false)
    }
}

impl fmt::Debug for dyn RuntimeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_debug_string())
    }
}

impl fmt::Display for dyn RuntimeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_display_string())
    }
}

/// Value is a boxed runtime value of any language-specific type.
pub type Value = Box<dyn RuntimeValue>;
