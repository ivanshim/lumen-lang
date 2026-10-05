// Single-threaded bindings following CPython v3.14.8 Python/context.c (PSF License).
// Context copies share values but own independent dictionaries. Numeric handles
// belong to this interpreter; Python protocol objects keep them private.
use crate::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

struct Saved {
    context: usize,
    variable: usize,
    previous: Option<Value>,
    used: bool,
}
struct Scope {
    bindings: HashMap<usize, Value>,
    entered: bool,
}
pub(crate) struct ContextStore {
    scopes: Vec<Scope>,
    variables: Vec<Value>,
    tokens: Vec<Saved>,
    current: usize,
}
impl Default for ContextStore {
    fn default() -> Self {
        Self { scopes: vec![Scope { bindings: HashMap::new(), entered: true }], variables: Vec::new(), tokens: Vec::new(), current: 0 }
    }
}
impl ContextStore {
    fn handle(args: &[Value], at: usize) -> Result<usize, String> {
        match args.get(at).map(Value::contents) {
            Some(Value::Small(n)) if n >= 0 => Ok(n as usize),
            _ => Err("TypeError: invalid context handle".into()),
        }
    }
    fn pair(value: Option<Value>) -> Value {
        Value::tuple(vec![Value::Flag(value.is_some()), value.unwrap_or(Value::Null)])
    }
    pub(crate) fn perform(&mut self, args: &[Value]) -> Result<Value, String> {
        let Some(Value::Text(op)) = args.first().map(Value::contents) else { return Err("TypeError: context operation must be a string".into()) };
        let index = if args.len() > 1 { Self::handle(args, 1)? } else { 0 };
        match op.as_ref() {
            "current" => Ok(Value::Small(self.current as i64)),
            "new" | "copy" => {
                let bindings = if op.as_ref() == "copy" { self.scopes.get(index).ok_or("ValueError: invalid Context")?.bindings.clone() } else { HashMap::new() };
                self.scopes.push(Scope { bindings, entered: false });
                Ok(Value::Small((self.scopes.len() - 1) as i64))
            }
            "var" => {
                // The first argument for creation is a placeholder handle.
                self.variables.push(args.get(2).ok_or("TypeError: missing ContextVar")?.clone());
                Ok(Value::Small((self.variables.len() - 1) as i64))
            }
            "enter" => {
                let scope = self.scopes.get_mut(index).ok_or("ValueError: invalid Context")?;
                if scope.entered { return Err(format!("RuntimeError: cannot enter context: {} is already entered", args.get(2).map(Value::plain).unwrap_or_default())); }
                scope.entered = true;
                let old = self.current;
                self.current = index;
                Ok(Value::Small(old as i64))
            }
            "leave" => {
                if index >= self.scopes.len() { return Err("ValueError: invalid Context".into()); }
                self.scopes[self.current].entered = false;
                self.current = index;
                Ok(Value::Null)
            }
            "get" => Ok(Self::pair(self.scopes[self.current].bindings.get(&index).cloned())),
            "lookup" => {
                let key = Self::handle(args, 2)?;
                Ok(Self::pair(self.scopes.get(index).ok_or("ValueError: invalid Context")?.bindings.get(&key).cloned()))
            }
            "set" => {
                if index >= self.variables.len() { return Err("ValueError: invalid ContextVar".into()); }
                let value = args.get(2).ok_or("TypeError: missing context value")?.clone();
                let previous = self.scopes[self.current].bindings.insert(index, value);
                self.tokens.push(Saved { context: self.current, variable: index, previous, used: false });
                Ok(Value::Small((self.tokens.len() - 1) as i64))
            }
            "token" => {
                let saved = self.tokens.get(index).ok_or("ValueError: invalid Token")?;
                Ok(Value::tuple(vec![self.variables[saved.variable].clone(), Self::pair(saved.previous.clone()), Value::Flag(saved.used)]))
            }
            "reset" => {
                let token = Self::handle(args, 2)?;
                let saved = self.tokens.get_mut(token).ok_or("ValueError: invalid Token")?;
                let repr = args.get(3).map(Value::plain).unwrap_or_default();
                if saved.used { return Err(format!("RuntimeError: {repr} has already been used once")); }
                if saved.variable != index { return Err(format!("ValueError: {repr} was created by a different ContextVar")); }
                if saved.context != self.current { return Err(format!("ValueError: {repr} was created in a different Context")); }
                saved.used = true;
                let bindings = &mut self.scopes[self.current].bindings;
                if let Some(old) = &saved.previous { bindings.insert(index, old.clone()); } else { bindings.remove(&index); }
                Ok(Value::Null)
            }
            "entries" => {
                let scope = self.scopes.get(index).ok_or("ValueError: invalid Context")?;
                let mut keys: Vec<_> = scope.bindings.keys().copied().collect();
                keys.sort_unstable();
                let entries: Vec<Value> = keys.into_iter().map(|key| Value::tuple(vec![self.variables[key].clone(), scope.bindings[&key].clone()])).collect();
                Ok(Value::Array(Rc::new(entries).into()))
            }
            _ => Err("ValueError: unknown context operation".into()),
        }
    }
}
