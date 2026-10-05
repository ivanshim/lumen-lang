// CPython v3.14.8 Python/context.c semantics (PSF License), for one thread.
// A context owns a row of bindings. Copying the row retains the same values;
// replacing a binding never changes another context's row.
use crate::data::Value;
use std::rc::Rc;

type Row = Vec<(usize, Value)>;
struct ResetPoint { owner: usize, key: usize, value: Option<Value>, consumed: bool }
pub(crate) struct Bindings {
    rows: Vec<Row>,
    active: Vec<bool>,
    names: Vec<Value>,
    resets: Vec<ResetPoint>,
    running: usize,
}
impl Bindings {
    pub(crate) fn new() -> Self {
        Bindings { rows: vec![Vec::new()], active: vec![true], names: Vec::new(), resets: Vec::new(), running: 0 }
    }
    fn number(input: &[Value], position: usize) -> Result<usize, String> {
        if let Some(value) = input.get(position) {
            if let Value::Small(n) = value.settled() {
                if n >= 0 { return Ok(n as usize); }
            }
        }
        Err("TypeError: invalid context handle".to_owned())
    }
    fn found(row: &Row, key: usize) -> Option<Value> {
        row.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
    }
    fn answer(value: Option<Value>) -> Value {
        let present = value.is_some();
        Value::tuple(vec![Value::Flag(present), value.unwrap_or(Value::Nil)])
    }
    pub(crate) fn apply(&mut self, input: &[Value]) -> Result<Value, String> {
        let operation = match input.first().map(Value::settled) {
            Some(Value::Text(text)) => text.to_string(),
            _ => return Err("TypeError: context operation must be a string".to_owned()),
        };
        let n = if input.len() < 2 { 0 } else { Self::number(input, 1)? };
        let result = match operation.as_str() {
            "current" => Value::Small(self.running as i64),
            "new" => {
                let next = self.rows.len();
                self.rows.push(Vec::new());
                self.active.push(false);
                Value::Small(next as i64)
            }
            "copy" => {
                let copied = self.rows.get(n).ok_or("ValueError: invalid Context")?.clone();
                let next = self.rows.len();
                self.rows.push(copied);
                self.active.push(false);
                Value::Small(next as i64)
            }
            "var" => {
                let named = input.get(2).ok_or("TypeError: missing ContextVar")?;
                self.names.push(named.clone());
                Value::Small((self.names.len() - 1) as i64)
            }
            "enter" => {
                let flag = self.active.get_mut(n).ok_or("ValueError: invalid Context")?;
                if *flag {
                    let shown = input.get(2).map(|v| v.settled().bare()).unwrap_or_default();
                    return Err(format!("RuntimeError: cannot enter context: {shown} is already entered"));
                }
                *flag = true;
                let previous = std::mem::replace(&mut self.running, n);
                Value::Small(previous as i64)
            }
            "leave" => {
                self.rows.get(n).ok_or("ValueError: invalid Context")?;
                self.active[self.running] = false;
                self.running = n;
                Value::Nil
            }
            "get" => Self::answer(Self::found(&self.rows[self.running], n)),
            "lookup" => {
                let variable = Self::number(input, 2)?;
                let row = self.rows.get(n).ok_or("ValueError: invalid Context")?;
                Self::answer(Self::found(row, variable))
            }
            "set" => {
                if self.names.get(n).is_none() { return Err("ValueError: invalid ContextVar".to_owned()); }
                let incoming = input.get(2).ok_or("TypeError: missing context value")?.clone();
                let row = &mut self.rows[self.running];
                let old = Self::found(row, n);
                row.retain(|(key, _)| *key != n);
                row.push((n, incoming));
                let number = self.resets.len();
                self.resets.push(ResetPoint { owner: self.running, key: n, value: old, consumed: false });
                Value::Small(number as i64)
            }
            "token" => {
                let point = self.resets.get(n).ok_or("ValueError: invalid Token")?;
                let parts = vec![self.names[point.key].clone(), Self::answer(point.value.clone()), Value::Flag(point.consumed)];
                Value::tuple(parts)
            }
            "reset" => {
                let serial = Self::number(input, 2)?;
                let point = self.resets.get_mut(serial).ok_or("ValueError: invalid Token")?;
                let display = input.get(3).map(|v| v.settled().bare()).unwrap_or_default();
                let complaint = if point.consumed {
                    Some(format!("RuntimeError: {display} has already been used once"))
                } else if n != point.key {
                    Some(format!("ValueError: {display} was created by a different ContextVar"))
                } else if self.running != point.owner {
                    Some(format!("ValueError: {display} was created in a different Context"))
                } else { None };
                if let Some(message) = complaint { return Err(message); }
                point.consumed = true;
                let row = &mut self.rows[self.running];
                row.retain(|(k, _)| *k != n);
                if let Some(value) = &point.value { row.push((n, value.clone())); }
                Value::Nil
            }
            "entries" => {
                let mut row = self.rows.get(n).ok_or("ValueError: invalid Context")?.clone();
                row.sort_by_key(|(key, _)| *key);
                let values: Vec<Value> = row.into_iter().map(|(key, value)| {
                    Value::tuple(vec![self.names[key].clone(), value])
                }).collect();
                Value::Vector(Rc::new(values).into())
            }
            _ => return Err("ValueError: unknown context operation".to_owned()),
        };
        Ok(result)
    }
}
