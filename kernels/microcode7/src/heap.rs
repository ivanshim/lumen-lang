//! The Python heap accelerator writes directly through a list's shared holder.
use std::{cell::RefCell, rc::Rc};
use super::{Machine, Prim, Value};

type HeapResult<T> = Result<T, String>;
struct HeapList(Rc<RefCell<Value>>);
impl HeapList {
    fn size(&self) -> usize {
        match &*self.0.borrow() { Value::Vector(values) => values.len(), _ => 0 }
    }
    fn read(&self, position: usize) -> HeapResult<Value> {
        let borrowed = self.0.borrow();
        if let Value::Vector(values) = &*borrowed {
            return values.get(position).cloned().ok_or_else(|| String::from("IndexError: index out of range"));
        }
        Err(String::from("TypeError: heap requires list storage"))
    }
    fn change<R>(&self, work: impl FnOnce(&mut Vec<Value>) -> R) -> HeapResult<R> {
        let mut borrowed = self.0.borrow_mut();
        let Value::Vector(values) = &mut *borrowed else { return Err(String::from("TypeError: heap requires list storage")) };
        Ok(work(Rc::make_mut(values)))
    }
    fn switch(&self, first: usize, next: usize) -> HeapResult<()> {
        if first >= self.size() || next >= self.size() { return Err(String::from("IndexError: index out of range")); }
        self.change(|row| row.swap(first, next))
    }
}
impl Machine<'_> {
    fn heap_order(&mut self, list: &HeapList, pair: (Value, Value), expected: usize) -> HeapResult<bool> {
        let answer = self.prim(Prim::Lt, "<", &[pair.0, pair.1])?;
        let accepted = self.object_truth(&answer)?;
        if list.size() == expected { Ok(accepted) }
        else { Err(String::from("RuntimeError: list changed size during iteration")) }
    }
    fn heap_towards_root(&mut self, list: &HeapList, bottom: usize, boundary: usize, backwards: bool) -> HeapResult<()> {
        let mut cursor = bottom;
        let expected = list.size();
        while cursor != boundary && cursor > boundary {
            let parent = (cursor - 1) >> 1;
            let low = list.read(cursor)?;
            let high = list.read(parent)?;
            let pair = if backwards { (high, low) } else { (low, high) };
            if !self.heap_order(list, pair, expected)? { return Ok(()); }
            list.switch(parent, cursor)?;
            cursor = parent;
        }
        Ok(())
    }
    fn heap_towards_leaf(&mut self, list: &HeapList, root: usize, backwards: bool) -> HeapResult<()> {
        let expected = list.size();
        let mut cursor = root;
        while (cursor << 1) + 1 < expected {
            let left = (cursor << 1) + 1;
            let mut chosen = left;
            if left + 1 < expected {
                let first = list.read(left)?;
                let second = list.read(left + 1)?;
                let pair = if backwards { (second, first) } else { (first, second) };
                chosen += usize::from(!self.heap_order(list, pair, expected)?);
            }
            list.switch(chosen, cursor)?;
            cursor = chosen;
        }
        self.heap_towards_root(list, cursor, root, backwards)
    }
    pub(super) fn heap_work(&mut self, given: &Value, operation: &str, newcomer: Option<Value>) -> HeapResult<Value> {
        let descending = operation.ends_with("_max");
        let action = operation.trim_end_matches("_max");
        let settled = given.settled();
        let kind = Value::Intrinsic(Prim::Listed, Rc::from("list"));
        if !self.core_belongs(&settled, &kind)? {
            let argument = if action == "heapify" || action == "heappop" { "argument" } else { "argument 1" };
            let description = match settled { Value::Nil => String::from("None"), _ => settled.kind_word() };
            return Err(format!("TypeError: {}() {} must be list, not {}", operation, argument, description));
        }
        let held = Self::underlying(&settled).unwrap_or_else(|| given.clone());
        let holder = Self::native_cell(&held).ok_or_else(|| String::from("TypeError: heap requires a list holder"))?;
        let list = HeapList(holder);
        let length = list.size();
        if action == "heapify" {
            let mut remaining = length >> 1;
            while remaining != 0 { remaining -= 1; self.heap_towards_leaf(&list, remaining, descending)?; }
            return Ok(Value::Nil);
        }
        if action == "heappush" {
            let value = newcomer.ok_or_else(|| String::from("TypeError: missing heap item"))?;
            list.change(|values| values.push(value))?;
            self.heap_towards_root(&list, length, 0, descending)?;
            return Ok(Value::Nil);
        }
        if action == "heappop" {
            let value = list.change(|values| values.pop())?.ok_or_else(|| String::from("IndexError: index out of range"))?;
            if length == 1 { return Ok(value); }
            let answer = list.read(0)?;
            list.change(|values| values[0] = value)?;
            self.heap_towards_leaf(&list, 0, descending)?;
            return Ok(answer);
        }
        if action == "heapreplace" || action == "heappushpop" {
            let value = newcomer.ok_or_else(|| String::from("TypeError: missing heap item"))?;
            if action == "heappushpop" {
                if length == 0 { return Ok(value); }
                let top = list.read(0)?;
                let pair = if descending { (value.clone(), top) } else { (top, value.clone()) };
                let response = self.prim(Prim::Lt, "<", &[pair.0, pair.1])?;
                if !self.object_truth(&response)? { return Ok(value); }
            }
            let answer = list.read(0)?;
            list.change(|values| values[0] = value)?;
            self.heap_towards_leaf(&list, 0, descending)?;
            return Ok(answer);
        }
        Err(format!("ValueError: unknown heap operation '{}'", operation))
    }
}
