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
    /// The index of each active call's first scope. Inside a call, lookup
    /// sees the call's own scopes and the global one, not the caller's.
    calls: Vec<usize>,
    extensions: HashMap<TypeId, Box<dyn Any>>,
}

impl Env {
    /// A new environment with a single (global) scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            calls: Vec::new(),
            extensions: HashMap::new(),
        }
    }

    /// Run `f` inside a fresh scope that is a call's own: it sees itself,
    /// the scopes it opens, and the global scope.
    pub fn with_call_scope<T>(&mut self, f: impl FnOnce(&mut Env) -> Result<T, String>) -> Result<T, String> {
        self.calls.push(self.scopes.len());
        let result = self.with_scope(f);
        self.calls.pop();
        result
    }

    /// The scopes in view, innermost first.
    fn in_view(&self) -> impl Iterator<Item = usize> {
        let floor = self.calls.last().copied().unwrap_or(0);
        let top = self.scopes.len();
        (floor..top).rev().chain((floor > 0).then_some(0))
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
        match self.get_mut(name) {
            Some(slot) => *slot = value,
            None => self.define(name.to_string(), value),
        }
    }

    /// Look up `name` from the innermost scope in view outward.
    pub fn get(&self, name: &str) -> Result<Value, String> {
        self.in_view()
            .find_map(|i| self.scopes[i].get(name))
            .cloned()
            .ok_or_else(|| format!("Undefined variable '{}'", name))
    }

    /// Mutable access to the innermost binding of `name` in view, for
    /// in-place mutation.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        let i = self.in_view().find(|&i| self.scopes[i].contains_key(name))?;
        self.scopes[i].get_mut(name)
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
