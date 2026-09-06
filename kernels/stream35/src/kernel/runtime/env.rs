// Runtime environment: lexical scopes holding opaque values.
//
// The kernel stores and retrieves values; it attaches no meaning to them.
// Languages that need runtime state beyond bindings (caches, registries,
// counters) keep it in a typed extension slot, so no language policy has to
// live in this file.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::kernel::runtime::Value;

pub struct Env {
    scopes: Vec<HashMap<String, Value>>,
    extensions: HashMap<TypeId, Box<dyn Any>>,
}

impl Env {
    /// A new environment with a single (global) scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            extensions: HashMap::new(),
        }
    }

    /// Enter a new lexical scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Leave the current scope. The global scope is never removed.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Run `f` inside a fresh scope that is popped on every exit path.
    pub fn with_scope<T>(&mut self, f: impl FnOnce(&mut Env) -> Result<T, String>) -> Result<T, String> {
        self.push_scope();
        let result = f(self);
        self.pop_scope();
        result
    }

    /// Bind `name` in the current scope, shadowing any outer binding.
    pub fn define(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Bind `name` in the current scope only (never touches outer scopes).
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        self.define(name.to_string(), value);
        Ok(())
    }

    /// Replace the innermost existing binding of `name`; if there is none,
    /// bind it in the current scope.
    pub fn update(&mut self, name: &str, value: Value) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(slot) = scope.get_mut(name) {
                *slot = value;
                return;
            }
        }
        self.define(name.to_string(), value);
    }

    /// Look up `name` from the innermost scope outward.
    pub fn get(&self, name: &str) -> Result<Value, String> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Ok(v.clone());
            }
        }
        Err(format!("Undefined variable '{}'", name))
    }

    /// Mutable access to the innermost binding of `name`, for in-place mutation.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(v) = scope.get_mut(name) {
                return Some(v);
            }
        }
        None
    }

    /// Typed side storage for language-defined runtime state.
    /// Created on first use from `T::default()`.
    pub fn extension<T: Default + 'static>(&mut self) -> &mut T {
        self.extensions
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut::<T>()
            .expect("extension slot holds the type it was created with")
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}
