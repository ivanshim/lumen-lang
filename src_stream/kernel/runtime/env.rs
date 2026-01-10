// src/framework/runtime/env.rs
//
// Runtime environment: variable bindings and lexical scopes.
// This file is core infrastructure and must remain stable.

use std::collections::HashMap;

use crate::kernel::runtime::Value;

// ============================================================================
// MEMOIZATION CACHE (SEMANTIC OPTIMIZATION LAYER)
// ============================================================================
//
// The cache is always created and present (matching microcode kernel design).
// Memoization is a language semantic decision, not a kernel feature.
// Only functions explicitly marked as memoizable use the cache.
//
// Cache key: (function_name, argument_fingerprint)
// The argument fingerprint is a stable representation of argument values.

type MemoKey = (String, String);

#[derive(Debug, Clone)]
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,

    // --- MEMOIZATION CACHE ---
    // Always created and present.
    // Only used when a function is explicitly marked as memoizable.
    // Cache is populated only for memoizable functions; other functions
    // perform no cache lookups or inserts.
    memoization_cache: HashMap<MemoKey, Value>,
}

impl Env {
    /// Create a new environment with a single (global) scope.
    /// The memoization cache is always created (matching microcode kernel design).
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            memoization_cache: HashMap::new(),
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

    // --- MEMOIZATION METHODS ---
    // Cache is always present. Only memoizable functions use these methods.

    /// Check if a result is cached for this function call.
    /// Returns Some(value) if cached, None if not in cache.
    pub fn get_cached(&self, func_name: &str, arg_fingerprint: &str) -> Option<Value> {
        let key = (func_name.to_string(), arg_fingerprint.to_string());
        self.memoization_cache.get(&key).cloned()
    }

    /// Cache the result of a function call.
    pub fn cache_result(&mut self, func_name: &str, arg_fingerprint: &str, result: Value) {
        let key = (func_name.to_string(), arg_fingerprint.to_string());
        self.memoization_cache.insert(key, result);
    }

    /// Generate a stable fingerprint from argument values.
    /// Used as part of the memoization cache key.
    pub fn fingerprint_args(args: &[Value]) -> String {
        args.iter()
            .map(|v| v.as_debug_string())
            .collect::<Vec<_>>()
            .join("|")
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}
