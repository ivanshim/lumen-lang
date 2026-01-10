// src/framework/runtime/env.rs
//
// Runtime environment: variable bindings and lexical scopes.
// This file is core infrastructure and must remain stable.

use std::collections::HashMap;

use crate::kernel::runtime::Value;

// ============================================================================
// MEMOIZATION CACHE & EXECUTION STATE
// ============================================================================
//
// MEMOIZATION is a system capability that enables/disables function result caching.
// It is:
// - Dynamically scoped (affects all calls made while enabled)
// - Inherited by callees
// - Automatically restored on scope exit
// - NOT a normal variable (reserved system identifier)
// - NOT readable, passable, or storable as data
//
// Cache key: (function_name, argument_fingerprint)

type MemoKey = (String, String);

#[derive(Debug, Clone)]
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,

    // --- MEMOIZATION STATE ---
    // Stack of memoization enabled/disabled states
    // Allows dynamic scoping with proper nesting
    memoization_stack: Vec<bool>,

    // --- MEMOIZATION CACHE ---
    // Function call result cache
    // Only populated when memoization_enabled() is true
    memoization_cache: HashMap<MemoKey, Value>,
}

impl Env {
    /// Create a new environment with a single (global) scope.
    /// Memoization is disabled by default (MEMOIZATION = false).
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            memoization_stack: vec![false],  // Default: MEMOIZATION = false
            memoization_cache: HashMap::new(),
        }
    }

    /// Check if memoization is currently enabled.
    pub fn memoization_enabled(&self) -> bool {
        self.memoization_stack.last().copied().unwrap_or(false)
    }

    /// Set memoization state (MEMOIZATION = true/false).
    /// This is dynamically scoped.
    pub fn set_memoization(&mut self, enabled: bool) {
        if let Some(last) = self.memoization_stack.last_mut() {
            *last = enabled;
        }
    }

    /// Push a new memoization state (for nested scopes).
    /// Called when entering a new scope.
    fn push_memoization_state(&mut self) {
        let current = self.memoization_enabled();
        self.memoization_stack.push(current);
    }

    /// Pop memoization state (for nested scopes).
    /// Called when exiting a scope.
    fn pop_memoization_state(&mut self) {
        if self.memoization_stack.len() > 1 {
            self.memoization_stack.pop();
        }
    }

    /// Enter a new lexical scope.
    /// Also preserves and manages memoization state for dynamic scoping.
    #[allow(dead_code)]
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.push_memoization_state();
    }

    /// Exit the current lexical scope.
    /// Also restores memoization state when exiting.
    #[allow(dead_code)]
    pub fn pop_scope(&mut self) {
        if self.scopes.len() == 1 {
            // Global scope must always exist.
            return;
        }
        self.scopes.pop();
        self.pop_memoization_state();
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

    // --- MEMOIZATION CACHE METHODS ---
    // Cache operations are gated by memoization_enabled() state.

    /// Check if a result is cached for this function call.
    /// Returns Some(value) only if memoization is enabled AND result is cached.
    pub fn get_cached(&self, func_name: &str, arg_fingerprint: &str) -> Option<Value> {
        if !self.memoization_enabled() {
            return None;
        }
        let key = (func_name.to_string(), arg_fingerprint.to_string());
        self.memoization_cache.get(&key).cloned()
    }

    /// Cache the result of a function call.
    /// Only caches if memoization is enabled.
    pub fn cache_result(&mut self, func_name: &str, arg_fingerprint: &str, result: Value) {
        if !self.memoization_enabled() {
            return;
        }
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
