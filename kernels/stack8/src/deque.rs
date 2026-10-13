//! Accelerate the collections adapter's two-stack storage without Python callbacks.
use std::{cell::RefCell, rc::Rc};
use super::{Engine, Res, Value};

type Cell = Rc<RefCell<Value>>;

// Read one list holder without retaining a copy of its row across a write.
fn cell_size(cell: &Cell) -> usize {
    match &*cell.borrow() { Value::Array(items) => items.len(), _ => 0 }
}

// Mutate a list in its existing cell so a live iterator retains its identity.
fn change<R>(cell: &Cell, work: impl FnOnce(&mut Vec<Value>) -> R) -> Res<R> {
    let mut held = cell.borrow_mut();
    let Value::Array(row) = &mut *held else { return Err("TypeError: deque storage must be a list".into()) };
    Ok(work(Rc::make_mut(row)))
}

// Take an endpoint, redistributing half the opposite stack only when needed.
fn take_end(right: &Cell, left: &Cell, front: bool) -> Res<Value> {
    let (destination, source) = if front { (left, right) } else { (right, left) };
    if cell_size(destination) == 0 {
        let count = cell_size(source).div_ceil(2);
        if count == 0 { return Err("IndexError: pop from an empty deque".into()); }
        let moved = change(source, |items| items.drain(..count).rev().collect::<Vec<_>>())?;
        change(destination, |items| items.extend(moved))?;
    }
    change(destination, |items| items.pop())?.ok_or_else(|| "IndexError: pop from an empty deque".into())
}

impl Engine<'_> {
    // Apply a storage operation after the Python adapter has checked public arguments.
    pub(super) fn deque_native_call(&mut self, storage: &Value, verb: &str, item: Option<Value>) -> Res<Value> {
        let outer = Self::holding_cell(storage).ok_or_else(|| "TypeError: deque storage must be a list".to_string())?;
        let (right_value, left_value, revision, limit) = match &*outer.borrow() {
            Value::Array(row) if row.len() == 4 => (row[0].clone(), row[1].clone(), row[2].clone(), row[3].clone()),
            _ => return Err("TypeError: invalid deque storage".into()),
        };
        let right = Self::holding_cell(&right_value).ok_or_else(|| "TypeError: invalid deque right stack".to_string())?;
        let left = Self::holding_cell(&left_value).ok_or_else(|| "TypeError: invalid deque left stack".to_string())?;
        let extent = cell_size(&right) + cell_size(&left);
        match verb {
            "length" => return Ok(Value::Small(extent as i64)),
            "state" => return Ok(revision),
            "limit" => return Ok(limit),
            "setstate" | "setlimit" => {
                let at = if verb == "setstate" { 2 } else { 3 };
                let value = item.ok_or_else(|| "TypeError: missing deque value".to_string())?;
                change(&outer, |row| row[at] = value)?;
                return Ok(Value::Null);
            }
            "data" => {
                if cell_size(&left) != 0 {
                    let mut prefix = change(&left, std::mem::take)?;
                    prefix.reverse();
                    change(&right, |row| { prefix.append(row); *row = prefix; })?;
                }
                return Ok(right_value);
            }
            _ => {}
        }
        let answer = match verb {
            "append" | "appendleft" => {
                let value = item.ok_or_else(|| "TypeError: missing deque item".to_string())?;
                change(if verb == "append" { &right } else { &left }, |row| row.push(value))?;
                if let Value::Small(maximum) = limit.contents() {
                    if extent >= maximum as usize { take_end(&right, &left, verb == "append")?; }
                }
                Value::Null
            }
            "pop" | "popleft" => take_end(&right, &left, verb == "popleft")?,
            "rotate" => {
                let number = match item.map(|v| v.contents()) { Some(Value::Small(n)) => n, _ => return Err("TypeError: invalid deque rotation".into()) };
                if extent <= 1 { return Ok(Value::Null); }
                let mut steps = number.rem_euclid(extent as i64);
                if steps > extent as i64 / 2 { steps -= extent as i64; }
                while steps != 0 {
                    let front = steps < 0;
                    let value = take_end(&right, &left, front)?;
                    change(if front { &right } else { &left }, |row| row.push(value))?;
                    steps += if front { 1 } else { -1 };
                }
                Value::Null
            }
            _ => return Err(format!("ValueError: unknown deque operation '{verb}'")),
        };
        let Value::Small(state) = revision.contents() else { return Err("TypeError: invalid deque state".into()) };
        change(&outer, |row| row[2] = Value::Small(state + 1))?;
        Ok(answer)
    }
}

impl Engine<'_> {
    // Fuse registered positional adapter calls without entering a Python frame.
    pub(super) fn deque_routine_call(&mut self, routine: &Rc<super::Routine>, given: &[Value]) -> Option<super::Flow<Value>> {
        let (_, descriptor, verb) = self.deque_routines.get(&(Rc::as_ptr(routine) as usize))?.clone();
        if !matches!(self.trace_hook, Value::Null) { return None; }
        if routine.revised.borrow().is_some() || self.recursion_ceiling().is_some_and(|n| self.calls.len() + self.reaching >= n) { return None; }
        let wanted = match verb.as_str() { "append" | "appendleft" | "setstate" | "setlimit" => 2, "rotate" => given.len().clamp(1, 2), "data" | "state" | "limit" | "length" | "pop" | "popleft" => 1, _ => return None };
        if given.len() != wanted || given.iter().any(|v| matches!(v, Value::Tie(_))) { return None; }
        let receiver = given[0].contents();
        let Value::Adapter(slot) = descriptor.contents() else { return None };
        let place = self.slot_place(&receiver, &slot.1).ok()?;
        let Value::Object(owner) = receiver else { return None };
        let storage = owner.fields.borrow().iter().find(|(key, _)| key == &place)?.1.clone();
        let item = if verb == "rotate" {
            match given.get(1).map(Value::contents) {
                None => Some(Value::Small(1)),
                Some(value @ Value::Small(_)) => Some(value),
                _ => return None,
            }
        } else { given.get(1).cloned() };
        Some(self.deque_native_call(&storage, &verb, item).map_err(Into::into))
    }
}
