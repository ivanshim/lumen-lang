// src/framework/runtime/env.rs
//
// Runtime environment: variable bindings and lexical scopes.
// This file is core infrastructure and must remain stable.

use std::collections::HashMap;

use crate::kernel::runtime::Value;

// ============================================================================
// MEMOIZATION CACHE (OPTIONAL OPTIMIZATION LAYER)
// ============================================================================
//
// This cache is created ONLY when memoization is explicitly enabled.
// When disabled (default), this remains None and has zero overhead.
//
// Cache key: (function_name, argument_fingerprint)
// The argument fingerprint is a stable representation of argument values.

type MemoKey = (String, String);

#[derive(Debug, Clone)]
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,

    // --- OPTIONAL OPTIMIZATION: Memoization cache ---
    // Created ONLY when enable_memoization is true in StreamExecutionOptions.
    // When None (default), no caching occurs and behavior is unchanged.
    memoization_cache: Option<HashMap<MemoKey, Value>>,
}

impl Env {
    /// Create a new environment with a single (global) scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            memoization_cache: None,
        }
    }

    /// Create an environment with memoization enabled.
    /// This is used when StreamExecutionOptions::enable_memoization is true.
    pub fn with_memoization() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            memoization_cache: Some(HashMap::new()),
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

    // --- OPTIONAL OPTIMIZATION: Memoization methods ---
    // These methods are called ONLY when memoization is enabled.
    // When memoization_cache is None (default), these have zero overhead.

    /// Check if a result is cached for this function call.
    /// Returns Some(value) if cached, None if not cached or memoization disabled.
    pub fn get_cached(&self, func_name: &str, arg_fingerprint: &str) -> Option<Value> {
        if let Some(cache) = &self.memoization_cache {
            let key = (func_name.to_string(), arg_fingerprint.to_string());
            cache.get(&key).cloned()
        } else {
            None
        }
    }

    /// Cache the result of a function call.
    /// This is a no-op if memoization is disabled.
    pub fn cache_result(&mut self, func_name: &str, arg_fingerprint: &str, result: Value) {
        if let Some(cache) = &mut self.memoization_cache {
            let key = (func_name.to_string(), arg_fingerprint.to_string());
            cache.insert(key, result);
        }
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
