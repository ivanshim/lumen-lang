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

    /// Define a new variable in the current scope.
    /// This shadows any outer binding with the same name.
    pub fn define(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Assign to a variable, searching parent scopes.
    /// If the variable exists in any enclosing scope, update it there.
    /// If not found, create it in the current scope.
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        // Variable not found in any scope; create in current scope.
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
        Ok(())
    }

    /// Internal: set a variable in the current scope only.
    /// Prefer assign() or define() in client code.
    #[allow(dead_code)]
    pub fn set(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
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
