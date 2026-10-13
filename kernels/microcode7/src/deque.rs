//! Keep each deque endpoint at the top of its own native list stack.
use std::cell::RefCell;
use std::rc::Rc;
use super::{Machine, Value};

type ResultOf<T> = Result<T, String>;
struct Stack(Rc<RefCell<Value>>);
impl Stack {
    // Count a stack while leaving its vector unshared.
    fn count(&self) -> usize {
        if let Value::Vector(values) = &*self.0.borrow() { values.len() } else { 0 }
    }
    // Write through the shared list identity used by the adapter and its iterators.
    fn edit<T>(&self, task: impl FnOnce(&mut Vec<Value>) -> T) -> ResultOf<T> {
        let mut borrowed = self.0.borrow_mut();
        match &mut *borrowed {
            Value::Vector(values) => Ok(task(Rc::make_mut(values))),
            _ => Err(String::from("TypeError: deque storage must be a list")),
        }
    }
    // Replenish an empty endpoint from half the other stack before removing it.
    fn remove(&self, opposite: &Stack) -> ResultOf<Value> {
        if self.count() == 0 {
            let n = opposite.count();
            if n == 0 { return Err(String::from("IndexError: pop from an empty deque")); }
            let portion = opposite.edit(|row| {
                let tail = row.split_off(n.div_ceil(2));
                let mut transferred = std::mem::replace(row, tail);
                transferred.reverse();
                transferred
            })?;
            self.edit(|row| row.extend(portion))?;
        }
        self.edit(Vec::pop)?.ok_or_else(|| String::from("IndexError: pop from an empty deque"))
    }
}
impl Machine<'_> {
    // Handle private storage commands; public validation remains in collections.
    pub(super) fn deque_work(&mut self, source: &Value, command: &str, supplied: Option<Value>) -> ResultOf<Value> {
        let holder = Self::native_cell(source).ok_or_else(|| String::from("TypeError: deque storage must be a list"))?;
        let state = Stack(holder);
        let parts = state.edit(|row| row.clone())?;
        if parts.len() != 4 { return Err(String::from("TypeError: invalid deque storage")); }
        let back = Stack(Self::native_cell(&parts[0]).ok_or_else(|| String::from("TypeError: invalid deque right stack"))?);
        let front = Stack(Self::native_cell(&parts[1]).ok_or_else(|| String::from("TypeError: invalid deque left stack"))?);
        let size = front.count() + back.count();
        if command == "length" { return Ok(Value::Small(size as i64)); }
        if command == "state" { return Ok(parts[2].clone()); }
        if command == "limit" { return Ok(parts[3].clone()); }
        if command.starts_with("set") {
            let slot = match command { "setstate" => 2, "setlimit" => 3, _ => return Err(String::from("ValueError: unknown deque setter")) };
            let value = supplied.ok_or_else(|| String::from("TypeError: missing deque value"))?;
            state.edit(|row| row[slot] = value)?;
            return Ok(Value::Nil);
        }
        if command == "data" {
            if front.count() > 0 {
                let prefix = front.edit(|row| { row.reverse(); std::mem::take(row) })?;
                back.edit(|row| {
                    let suffix = std::mem::replace(row, prefix);
                    row.extend(suffix);
                })?;
            }
            return Ok(parts[0].clone());
        }
        let mut result = Value::Nil;
        if command == "append" || command == "appendleft" {
            let value = supplied.ok_or_else(|| String::from("TypeError: missing deque item"))?;
            let to_front = command == "appendleft";
            (if to_front { &front } else { &back }).edit(|row| row.push(value))?;
            if let Value::Small(bound) = parts[3].settled() {
                if size >= bound as usize {
                    if to_front { back.remove(&front)?; } else { front.remove(&back)?; }
                }
            }
        } else if command == "pop" {
            result = back.remove(&front)?;
        } else if command == "popleft" {
            result = front.remove(&back)?;
        } else if command == "rotate" {
            if size < 2 { return Ok(result); }
            let n = match supplied.map(|v| v.settled()) { Some(Value::Small(n)) => n, _ => return Err(String::from("TypeError: invalid deque rotation")) };
            let forward = n.rem_euclid(size as i64) as usize;
            if forward <= size / 2 {
                for _ in 0..forward {
                    let next = back.remove(&front)?;
                    front.edit(|row| row.push(next))?;
                }
            } else {
                for _ in forward..size {
                    let next = front.remove(&back)?;
                    back.edit(|row| row.push(next))?;
                }
            }
        } else {
            return Err(format!("ValueError: unknown deque operation '{}'", command));
        }
        let version = match parts[2].settled() { Value::Small(n) => n, _ => return Err(String::from("TypeError: invalid deque state")) };
        state.edit(|row| row[2] = Value::Small(version + 1))?;
        Ok(result)
    }
}
