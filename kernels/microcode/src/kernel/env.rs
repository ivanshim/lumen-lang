// Environment: a stack of binding scopes plus the function-result cache.

use std::collections::HashMap;

use crate::kernel::value::Value;

type Scope = HashMap<String, Value>;

pub struct Environment {
    scopes: Vec<Scope>,
    /// (function name, argument fingerprint) → result. Consulted only while
    /// the schema-named memoization binding evaluates to true.
    call_cache: HashMap<(String, String), Value>,
}

impl Environment {
    pub fn new() -> Self {
        Environment { scopes: vec![HashMap::new()], call_cache: HashMap::new() }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Run `f` in a fresh scope that is popped on every exit path.
    pub fn with_scope<T>(&mut self, f: impl FnOnce(&mut Environment) -> Result<T, String>) -> Result<T, String> {
        self.push_scope();
        let result = f(self);
        self.pop_scope();
        result
    }

    /// Bind in the current scope.
    pub fn define(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Look up from the innermost scope outward.
    pub fn get(&self, name: &str) -> Result<Value, String> {
        self.lookup(name).cloned().ok_or_else(|| format!("Undefined variable: {}", name))
    }

    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.scopes.iter_mut().rev().find_map(|scope| scope.get_mut(name))
    }

    pub fn cached_result(&self, key: &(String, String)) -> Option<Value> {
        self.call_cache.get(key).cloned()
    }

    pub fn cache_result(&mut self, key: (String, String), result: Value) {
        self.call_cache.insert(key, result);
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
