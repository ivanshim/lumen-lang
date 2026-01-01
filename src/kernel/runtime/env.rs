// src/framework/runtime/env.rs
//
// Runtime environment: variable bindings and lexical scopes.
// This file is core infrastructure and must remain stable.

use std::collections::HashMap;

use crate::kernel::runtime::Value;

#[derive(Debug, Clone)]
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,
}

impl Env {
    /// Create a new environment with a single (global) scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Enter a new lexical scope.
    #[allow(dead_code)]
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Exit the current lexical scope.
    #[allow(dead_code)]
    pub fn pop_scope(&mut self) {
        if self.scopes.len() == 1 {
            // Global scope must always exist.
            return;
        }
        self.scopes.pop();
    }

    /// Define or overwrite a variable in the current scope.
    pub fn set(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Assign to an existing variable.
    /// Walks outward through scopes.
    #[allow(dead_code)]
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        Err(format!("Undefined variable '{}'", name))
    }

    /// Retrieve a variable value.
    pub fn get(&self, name: &str) -> Result<Value, String> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Ok(v.clone());
            }
        }
        Err(format!("Undefined variable '{}'", name))
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}
