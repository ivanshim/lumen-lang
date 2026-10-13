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

// Recognize immutable native scalars, excluding all user-defined subclasses.
fn deque_scalar(value: &Value) -> bool {
    matches!(value.contents(), Value::Small(_) | Value::Huge(_) | Value::Real(_)
        | Value::Flag(_) | Value::Text(_) | Value::Codepoints(_) | Value::Complex(_)
        | Value::Null | Value::Bytes(_, false, _))
}

impl Engine<'_> {
    // Apply a storage operation after the Python adapter has checked public arguments.
    pub(super) fn deque_native_call(&mut self, storage: &Value, verb: &str, item: Option<Value>) -> Res<Value> {
        if verb == "plain" {
            let value = item.ok_or_else(|| "TypeError: missing deque comparison value".to_string())?;
            let result = match storage.contents() {
                Value::Array(row) => deque_scalar(&value) && row.iter().all(deque_scalar),
                _ => return Err("TypeError: deque scan storage must be a list".into()),
            };
            return Ok(Value::Flag(result));
        }
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
        if matches!(verb.as_str(), "next" | "reverse_next") {
            if given.len() != 1 || matches!(given[0], Value::Tie(_)) { return None; }
            return self.deque_iterator_step(&given[0].contents(), &descriptor, verb == "reverse_next");
        }
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

impl Engine<'_> {
    // Read an owned slot while its descriptor and default attribute hooks remain intact.
    fn deque_iterator_slot(&self, target: &Value, descriptor: &Value) -> Option<(String, Value)> {
        let Value::Adapter(slot) = descriptor else { return None };
        let Value::Class(class) = &slot.1[1] else { return None };
        let members = class.shared.borrow();
        if members.iter().any(|(name, _)| matches!(name.as_str(), "__getattribute__" | "__getattr__" | "__setattr__")) { return None; }
        let original = members.iter().find(|(name, _)| name == &slot.1[0].plain())?.1.contents();
        if !matches!(original, Value::Adapter(ref current) if Rc::ptr_eq(current, slot)) { return None; }
        drop(members);
        let key = self.slot_place(target, &slot.1).ok()?;
        let Value::Object(object) = target else { return None };
        let value = object.fields.borrow().iter().find(|(name, _)| name == &key)?.1.clone();
        Some((key, value))
    }

    // Advance an exact deque iterator with the same remaining-count and mutation rules.
    fn deque_iterator_step(&mut self, target: &Value, descriptors: &Value, reverse: bool) -> Option<super::Flow<Value>> {
        let Value::Tuple(slots) = descriptors.contents() else { return None };
        let Value::Object(iterator) = target else { return None };
        let Value::Adapter(first) = &slots[0] else { return None };
        if !matches!(&first.1[1], Value::Class(class) if Rc::ptr_eq(&iterator.class_now(), class)) { return None; }
        let (_, data) = self.deque_iterator_slot(target, &slots[0])?;
        let (_, owner) = self.deque_iterator_slot(target, &slots[1])?;
        let (_, saved) = self.deque_iterator_slot(target, &slots[2])?;
        let (remaining_key, left) = self.deque_iterator_slot(target, &slots[3])?;
        let (_, total) = self.deque_iterator_slot(target, &slots[4])?;
        let (Value::Small(remaining), Value::Small(size), Value::Small(version)) = (left.contents(), total.contents(), saved.contents()) else { return None };
        if remaining < 0 || size < remaining { return None; }
        let stop = "StopIteration: ";
        if reverse && remaining == 0 { return Some(self.raised_of_note(Err(stop.into()))); }
        let (_, storage) = self.deque_iterator_slot(&owner.contents(), &slots[5])?;
        let current = self.deque_native_call(&storage, "state", None).ok()?;
        let Value::Small(now) = current else { return None };
        if now != version {
            iterator.fields.borrow_mut().iter_mut().find(|(key, _)| key == &remaining_key)?.1 = Value::Small(0);
            return Some(self.raised_of_note(Err("RuntimeError: deque mutated during iteration".into())));
        }
        if remaining == 0 { return Some(self.raised_of_note(Err(stop.into()))); }
        let Value::Array(items) = data.contents() else { return None };
        let index = if reverse { remaining - 1 } else { size - remaining } as usize;
        let value = items.get(index)?.clone();
        iterator.fields.borrow_mut().iter_mut().find(|(key, _)| key == &remaining_key)?.1 = Value::Small(remaining - 1);
        Some(Ok(value))
    }
}
