// Environment: a linear binding stack with frame markers.
//
// Bindings are pushed in order; a frame records where it began and leaving
// it truncates the stack back to that point. Lookup scans from the youngest
// binding down, so an inner binding shadows an outer one. A call also
// records where it began: inside it, lookup sees the call's own bindings
// (its blocks included) and the globals, the bindings made before any
// frame, and not the caller's. The call cache holds memoised function
// results.

use std::collections::HashMap;

use crate::kernel::value::Value;

pub struct Environment {
    bindings: Vec<(String, Value)>,
    frames: Vec<usize>,
    calls: Vec<usize>,
    call_cache: HashMap<(String, String), Value>,
}

impl Environment {
    pub fn new() -> Self {
        Environment { bindings: Vec::new(), frames: Vec::new(), calls: Vec::new(), call_cache: HashMap::new() }
    }

    /// Run `f` inside a call's frame: what it binds is its own, and what
    /// it sees is its own and the globals.
    pub fn in_call<T>(&mut self, f: impl FnOnce(&mut Environment) -> Result<T, String>) -> Result<T, String> {
        self.calls.push(self.bindings.len());
        let outcome = self.in_frame(f);
        self.calls.pop();
        outcome
    }

    /// Whether the binding at `index` is in view: the innermost call's own
    /// bindings, or the globals, which lie before the first frame.
    fn visible(&self, index: usize) -> bool {
        match self.calls.last() {
            Some(&own) => index >= own || index < self.frames.first().copied().unwrap_or(0),
            None => true,
        }
    }

    fn position(&self, name: &str, wanted: impl Fn(&Value) -> bool) -> Option<usize> {
        (0..self.bindings.len()).rev().find(|&i| self.bindings[i].0 == name && self.visible(i) && wanted(&self.bindings[i].1))
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

    /// The youngest binding of `name` in view, if any.
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.position(name, |_| true).map(|i| &self.bindings[i].1)
    }

    /// The youngest binding of `name` that holds a function: a language
    /// whose functions return by assigning to their own name (Pascal) may
    /// shadow the function with that variable inside its body.
    pub fn lookup_function(&self, name: &str) -> Option<&Value> {
        self.position(name, |v| matches!(v, Value::Function(_))).map(|i| &self.bindings[i].1)
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Value> {
        let index = self.position(name, |_| true)?;
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
