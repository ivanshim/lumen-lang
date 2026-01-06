// Environment: scope and binding management
//
// Minimal, explicit scope stack.
// No special semantics - just name lookup.

use crate::kernel::eval::Value;
use std::collections::HashMap;

/// A single scope frame
type Scope = HashMap<String, Value>;

/// Environment: stack of scopes
/// Top of stack is current scope.
pub struct Environment {
    scopes: Vec<Scope>,
}

impl Environment {
    /// Create new environment with global scope
    pub fn new() -> Self {
        Environment {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Set binding in current scope
    pub fn set(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Get binding (search from current scope upward)
    pub fn get(&self, name: &str) -> Result<Value, String> {
        // Search from top to bottom
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value.clone());
            }
        }
        Err(format!("Undefined variable: {}", name))
    }

    /// Check if name exists in any scope
    pub fn exists(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains_key(name))
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
