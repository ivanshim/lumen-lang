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
// Refuse objects and mutable byte arrays before delegating comparison to a list.
fn scan_scalar(item: &Value) -> bool {
    match item.settled() {
        Value::Nil | Value::Flag(_) | Value::Small(_) | Value::Huge(_) => true,
        Value::Text(_) | Value::Unpaired(_) | Value::Complex(_) => true,
        Value::Frac(number) => number.places.is_some(),
        Value::Octets { changeable: false, .. } => true,
        _ => false,
    }
}

impl Machine<'_> {
    // Handle private storage commands; public validation remains in collections.
    pub(super) fn deque_work(&mut self, source: &Value, command: &str, supplied: Option<Value>) -> ResultOf<Value> {
        if command == "plain" {
            let sought = supplied.ok_or_else(|| String::from("TypeError: missing deque comparison value"))?;
            let Value::Vector(items) = source.settled() else { return Err(String::from("TypeError: deque scan storage must be a list")) };
            return Ok(Value::Flag(scan_scalar(&sought) && items.iter().all(scan_scalar)));
        }
        let holder = Self::native_cell(source).ok_or_else(|| String::from("TypeError: deque storage must be a list"))?;
        let state = Stack(holder);
        let parts = state.edit(|row| row.clone())?;
        if parts.len() != 4 { return Err(String::from("TypeError: invalid deque storage")); }
        let back = Stack(Self::native_cell(&parts[0]).ok_or_else(|| String::from("TypeError: invalid deque right stack"))?);
        let front = Stack(Self::native_cell(&parts[1]).ok_or_else(|| String::from("TypeError: invalid deque left stack"))?);
        let size = front.count() + back.count();
        if command == "getitem" {
            let index = match supplied.map(|value| value.settled()) { Some(Value::Small(n)) => n, _ => return Err(String::from("TypeError: invalid deque subscript")) };
            let position = if index < 0 { index.saturating_add(size as i64) } else { index };
            if !(0..size as i64).contains(&position) { return Err(String::from("IndexError: deque index out of range")); }
            let n = front.count();
            let (selected, place) = if position as usize >= n { (&back, position as usize - n) } else { (&front, n - position as usize - 1) };
            let contents = selected.0.borrow();
            return match &*contents {
                Value::Vector(items) => Ok(items[place].clone()),
                _ => Err(String::from("TypeError: invalid deque stack")),
            };
        }
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

impl Machine<'_> {
    // Accelerate only a registered function's untouched positional call shape.
    pub(super) fn deque_function_call(&mut self, body: &Rc<super::Routine>, room: &Rc<super::Env>, values: &[Value]) -> Option<super::Res> {
        let identity = (Rc::as_ptr(body) as usize, Rc::as_ptr(room) as usize);
        let (original, descriptor, command) = self.deque_functions.get(&identity.0)?.clone();
        if !matches!(self.tracing_function, Value::Nil) { return None; }
        if let Value::Bound(_, captured) = original {
            if self.written_over.contains_key(&(identity.0, Rc::as_ptr(&captured) as usize)) { return None; }
        }
        if self.written_over.contains_key(&identity) { return None; }
        if self.recursion_ceiling().is_some_and(|ceiling| self.standing >= ceiling) { return None; }
        if command == "next" || command == "reverse_next" {
            if values.len() != 1 || matches!(values[0], Value::Couple(_)) { return None; }
            return self.deque_iterator_take(values[0].settled(), descriptor, command == "reverse_next");
        }
        let shape = match command.as_str() {
            "append" | "appendleft" | "setlimit" | "setstate" | "getitem" => values.len() == 2,
            "rotate" => values.len() == 1 || values.len() == 2,
            "index" => (2..=4).contains(&values.len()),
            "pop" | "popleft" | "data" | "state" | "limit" | "length" => values.len() == 1,
            _ => false,
        };
        if !shape || values.iter().any(|v| matches!(v, Value::Couple(_))) { return None; }
        let target = values[0].settled();
        let Value::Wrapped(32, detail) = descriptor else { return None };
        let address = self.slot_key(&target, &detail).ok()?;
        let stored = if let Value::Thing(object) = target {
            object.holds.borrow().iter().find_map(|(name, value)| (name == &address).then(|| value.clone()))?
        } else { return None };
        if command == "index" { return self.deque_index_lookup(&stored, values); }
        if command == "getitem" {
            let subscript = values[1].settled();
            if !matches!(subscript, Value::Small(_)) { return None; }
            let answer = self.deque_work(&stored, &command, Some(subscript)).map_err(Into::into);
            return Some(self.raised_if_error(answer));
        }
        let argument = if command == "rotate" {
            if values.len() == 1 { Some(Value::Small(1)) }
            else if let next @ Value::Small(_) = values[1].settled() { Some(next) }
            else { return None }
        } else { values.get(1).cloned() };
        Some(self.deque_work(&stored, &command, argument).map_err(Into::into))
    }
}

impl Machine<'_> {
    // Resolve unchanged slot descriptors without using overridable attribute access.
    fn deque_walk_field(&self, object: &Value, slot: &Value) -> Option<(String, Value)> {
        let Value::Wrapped(32, detail) = slot else { return None };
        let Value::Blueprint(declared) = &detail[1] else { return None };
        let entries = declared.shared.borrow();
        for (name, _) in entries.iter() {
            if name == "__getattr__" || name == "__setattr__" || name == "__getattribute__" { return None; }
        }
        let current = entries.iter().find_map(|(name, value)| (name == &detail[0].bare()).then(|| value.settled()))?;
        if !matches!(current, Value::Wrapped(32, ref held) if Rc::ptr_eq(held, detail)) { return None; }
        drop(entries);
        let address = self.slot_key(object, detail).ok()?;
        if let Value::Thing(instance) = object {
            let found = instance.holds.borrow().iter().find_map(|(key, value)| (key == &address).then(|| value.clone()))?;
            Some((address, found))
        } else { None }
    }

    // Take one item, retaining the forward and reverse exhaustion order of the adapter.
    fn deque_iterator_take(&mut self, object: Value, layout: Value, backwards: bool) -> Option<super::Res> {
        let Value::Tuple(fields) = layout else { return None };
        let Value::Thing(instance) = &object else { return None };
        let Value::Wrapped(32, declaration) = &fields[0] else { return None };
        let Value::Blueprint(expected) = &declaration[1] else { return None };
        if !Rc::ptr_eq(expected, &instance.blueprint()) { return None; }
        let captured = fields[..5].iter().map(|slot| self.deque_walk_field(&object, slot)).collect::<Option<Vec<_>>>()?;
        let mut numbers = [0i64; 3];
        for (destination, position) in [2, 3, 4].into_iter().enumerate() {
            if let Value::Small(number) = captured[position].1.settled() { numbers[destination] = number; } else { return None; }
        }
        let [version, count, extent] = numbers;
        if count < 0 || count > extent { return None; }
        if backwards && count == 0 { return Some(self.raised_if_error(Err(String::from("StopIteration: ").into()))); }
        let owner = captured[1].1.settled();
        let (_, state) = self.deque_walk_field(&owner, &fields[5])?;
        let present = self.deque_work(&state, "state", None).ok()?;
        let Value::Small(latest) = present else { return None };
        let remaining = &captured[3].0;
        if version != latest {
            instance.holds.borrow_mut().iter_mut().find(|(name, _)| name == remaining)?.1 = Value::Small(0);
            return Some(self.raised_if_error(Err(String::from("RuntimeError: deque mutated during iteration").into())));
        }
        if count == 0 { return Some(self.raised_if_error(Err(String::from("StopIteration: ").into()))); }
        let Value::Vector(row) = captured[0].1.settled() else { return None };
        let offset = if backwards { count - 1 } else { extent - count };
        let result = row.get(offset as usize)?.clone();
        instance.holds.borrow_mut().iter_mut().find(|(name, _)| name == remaining)?.1 = Value::Small(count - 1);
        Some(Ok(result))
    }
}

impl Machine<'_> {
    // Keep arbitrary conversion and equality hooks in Python; scan native scalars here.
    fn deque_index_lookup(&mut self, state: &Value, args: &[Value]) -> Option<super::Res> {
        let sought = match args[1].settled() { Value::Flag(flag) => Value::Small(flag as i64), other => other };
        if !scan_scalar(&sought) { return None; }
        let bounds = args[2..].iter().map(|v| match v.settled() { Value::Small(n) => Some(n), _ => None }).collect::<Option<Vec<_>>>()?;
        let data = self.deque_work(state, "data", None).ok()?;
        let Value::Vector(items) = data.settled() else { return None };
        if !items.iter().all(scan_scalar) { return None; }
        let normalize = |bound: i64| -> usize {
            let n = if bound < 0 { (items.len() as i64).saturating_add(bound).max(0) } else { bound };
            (n as usize).min(items.len())
        };
        let first = bounds.first().map_or(0, |n| normalize(*n));
        let last = bounds.get(1).map_or(items.len(), |n| normalize(*n));
        let found = (first..last).find(|position| {
            let item = match items[*position].settled() { Value::Flag(flag) => Value::Small(flag as i64), other => other };
            let same = match (&item, &sought) { (Value::Complex(left), Value::Complex(right)) => Rc::ptr_eq(left, right), _ => item.one_place(&sought) };
            same || item.equals(&sought)
        });
        match found {
            Some(position) => Some(Ok(Value::Small(position as i64))),
            None => Some(self.raised_if_error(Err(String::from("ValueError: deque.index(x): x not in deque").into()))),
        }
    }
}
