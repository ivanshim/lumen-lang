// Environment: a linear binding stack with frame markers.
//
// Bindings are pushed in order; a frame records where it began and leaving
// it truncates the stack back to that point. Lookup scans from the youngest
// binding down, so an inner binding shadows an outer one and a callee sees
// its caller's bindings for as long as the call lasts. The call cache holds
// memoised function results.

use std::collections::HashMap;

use crate::kernel::value::Value;

pub struct Environment {
    bindings: Vec<(String, Value)>,
    frames: Vec<usize>,
    call_cache: HashMap<(String, String), Value>,
}

impl Environment {
    pub fn new() -> Self {
        Environment { bindings: Vec::new(), frames: Vec::new(), call_cache: HashMap::new() }
    }

    fn frame_start(&self) -> usize {
        self.frames.last().copied().unwrap_or(0)
    }

    pub fn enter_frame(&mut self) {
        self.frames.push(self.bindings.len());
    }

    pub fn leave_frame(&mut self) {
        if let Some(start) = self.frames.pop() {
            self.bindings.truncate(start);
        }
    }

    /// Run `f` inside a frame that is left on every exit path.
    pub fn in_frame<T>(&mut self, f: impl FnOnce(&mut Environment) -> Result<T, String>) -> Result<T, String> {
        self.enter_frame();
        let outcome = f(self);
        self.leave_frame();
        outcome
    }

    /// Bind `name` in the current frame, replacing an earlier binding of the
    /// same name made in this frame.
    pub fn bind(&mut self, name: String, value: Value) {
        let start = self.frame_start();
        match self.bindings[start..].iter().rposition(|(n, _)| *n == name) {
            Some(offset) => self.bindings[start + offset].1 = value,
            None => self.bindings.push((name, value)),
        }
    }

    /// The youngest binding of `name`, if any.
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.bindings.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Value> {
        let index = self.bindings.iter().rposition(|(n, _)| n == name)?;
        Some(&mut self.bindings[index].1)
    }

    /// A copy of the youngest binding of `name`, or an error naming it.
    pub fn value(&self, name: &str) -> Result<Value, String> {
        self.lookup(name).cloned().ok_or_else(|| format!("Undefined variable: {}", name))
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
