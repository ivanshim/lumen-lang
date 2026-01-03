// Execution environment with lexical scoping
//
// The environment manages variable bindings and provides scope isolation.
// Scopes are created for blocks and restored on exit.

use crate::src_microcode::kernel::eval::Value;
use std::collections::HashMap;

/// Execution environment with lexically-scoped variable bindings
pub struct Environment {
    /// Stack of scopes, where each scope is a HashMap of variable bindings
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    /// Create a new environment with a global scope
    pub fn new() -> Self {
        let mut env = Self {
            scopes: Vec::new(),
        };
        env.push_scope();
        env
    }

    /// Push a new scope onto the stack
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope from the stack
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Set a variable in the current scope
    pub fn set(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Get a variable, searching from innermost to outermost scope
    pub fn get(&self, name: &str) -> Result<Value, String> {
        // Search from innermost (last) to outermost (first) scope
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value.clone());
            }
        }
        Err(format!("Undefined variable: {}", name))
    }

    /// Update a variable if it exists in any scope, otherwise error
    pub fn update(&mut self, name: String, value: Value) -> Result<(), String> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(&name) {
                scope.insert(name, value);
                return Ok(());
            }
        }
        Err(format!("Undefined variable: {}", name))
    }

    /// Define a variable in the current scope (shadowing any outer definition)
    pub fn define(&mut self, name: String, value: Value) {
        self.set(name, value);
    }

    /// Get number of scopes (for debugging)
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_isolation() {
        let mut env = Environment::new();

        env.set("x".to_string(), Value::Number(1.0));
        assert_eq!(env.get("x").unwrap(), Value::Number(1.0));

        env.push_scope();
        env.set("x".to_string(), Value::Number(2.0));
        assert_eq!(env.get("x").unwrap(), Value::Number(2.0));

        env.pop_scope();
        assert_eq!(env.get("x").unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_undefined_variable() {
        let env = Environment::new();
        assert!(env.get("undefined").is_err());
    }
}
