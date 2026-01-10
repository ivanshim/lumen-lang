// Environment: scope and binding management
//
// Minimal, explicit scope stack.
// No special semantics - just name lookup.

use crate::kernel::eval::Value;
use crate::kernel::primitives::Instruction;
use std::collections::HashMap;

/// Metadata about a function
#[derive(Clone, Debug)]
pub struct FunctionMetadata {
    pub params: Vec<String>,
    pub body: Instruction,
    pub memoizable: bool,
}

/// Cache key: (function_name, argument_hashes)
/// Using hashes of arguments for stable key generation
type CacheKey = (String, String);

/// A single scope frame
type Scope = HashMap<String, Value>;

/// Environment: stack of scopes
/// Top of stack is current scope.
pub struct Environment {
    scopes: Vec<Scope>,
    /// Store function metadata (params, body, memoizable flag)
    pub functions: HashMap<String, FunctionMetadata>,
    /// Call cache: (function_name, argument_values_repr) -> result
    /// Only populated for memoizable functions
    call_cache: HashMap<CacheKey, Value>,
}

impl Environment {
    /// Create new environment with global scope
    pub fn new() -> Self {
        Environment {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            call_cache: HashMap::new(),
        }
    }

    /// Get cached result for a function call (if memoizable and cached)
    pub fn get_cached(&self, func_name: &str, args: &[Value]) -> Option<Value> {
        let cache_key = (func_name.to_string(), Self::args_to_key(args));
        self.call_cache.get(&cache_key).cloned()
    }

    /// Cache a function result (only called for memoizable functions)
    pub fn cache_result(&mut self, func_name: &str, args: &[Value], result: Value) {
        let cache_key = (func_name.to_string(), Self::args_to_key(args));
        self.call_cache.insert(cache_key, result);
    }

    /// Generate a stable cache key from argument values
    fn args_to_key(args: &[Value]) -> String {
        // Use Debug format as stable string representation
        // This is deterministic and reversible
        args.iter()
            .map(|v| format!("{:?}", v))
            .collect::<Vec<_>>()
            .join("|")
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
