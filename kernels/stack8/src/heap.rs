//! List-backed heap operations for the Python accelerator.
use std::cell::RefCell;
use std::rc::Rc;
use super::{Action, Builtin, Engine, Res, Value};

impl Engine<'_> {
    fn heap_row_len(cell: &Rc<RefCell<Value>>) -> usize {
        if let Value::Array(row) = &*cell.borrow() { row.len() } else { 0 }
    }
    fn heap_row_item(cell: &Rc<RefCell<Value>>, index: usize) -> Res<Value> {
        match &*cell.borrow() {
            Value::Array(row) => row.get(index).cloned().ok_or_else(|| "IndexError: index out of range".into()),
            _ => Err("TypeError: heap storage is not a list".into()),
        }
    }
    fn heap_exchange(cell: &Rc<RefCell<Value>>, first: usize, second: usize) -> Res<()> {
        let mut held = cell.borrow_mut();
        let Value::Array(row) = &mut *held else { return Err("TypeError: heap storage is not a list".into()) };
        if first >= row.len() || second >= row.len() { return Err("IndexError: index out of range".into()); }
        Rc::make_mut(row).swap(first, second);
        Ok(())
    }
    fn heap_less(&mut self, cell: &Rc<RefCell<Value>>, a: Value, b: Value, extent: usize) -> Res<bool> {
        let answer = self.special_dyad(&Action::Lt, &a, &b)?;
        let yes = self.special_truth(&answer)?;
        if Self::heap_row_len(cell) != extent { return Err("RuntimeError: list changed size during iteration".into()); }
        Ok(yes)
    }
    fn heap_climb(&mut self, cell: &Rc<RefCell<Value>>, lower: usize, mut at: usize, maximum: bool) -> Res<()> {
        let extent = Self::heap_row_len(cell);
        while at > lower {
            let above = (at - 1) / 2;
            let child = Self::heap_row_item(cell, at)?;
            let parent = Self::heap_row_item(cell, above)?;
            let (one, two) = if maximum { (parent, child) } else { (child, parent) };
            if !self.heap_less(cell, one, two, extent)? { break; }
            Self::heap_exchange(cell, at, above)?;
            at = above;
        }
        Ok(())
    }
    fn heap_sink(&mut self, cell: &Rc<RefCell<Value>>, mut at: usize, maximum: bool) -> Res<()> {
        let start = at;
        let extent = Self::heap_row_len(cell);
        loop {
            let mut child = at * 2 + 1;
            if child >= extent { break; }
            if child + 1 < extent {
                let left = Self::heap_row_item(cell, child)?;
                let right = Self::heap_row_item(cell, child + 1)?;
                let (one, two) = if maximum { (right, left) } else { (left, right) };
                if !self.heap_less(cell, one, two, extent)? { child += 1; }
            }
            Self::heap_exchange(cell, at, child)?;
            at = child;
        }
        self.heap_climb(cell, start, at, maximum)
    }
    pub(super) fn heap_native_call(&mut self, heap: &Value, operation: &str, item: Option<Value>) -> Res<Value> {
        let maximum = operation.ends_with("_max");
        let verb = operation.strip_suffix("_max").unwrap_or(operation);
        let supplied = heap.contents();
        if !self.core_isinstance(&supplied, &Value::Native(Builtin::List, Rc::from("list")))? {
            let position = if matches!(verb, "heappush" | "heappushpop" | "heapreplace") { "argument 1" } else { "argument" };
            let kind = if matches!(supplied, Value::Null) { "None".to_string() } else { supplied.core_kind() };
            return Err(format!("TypeError: {operation}() {position} must be list, not {kind}"));
        }
        let storage = Self::worth_of(&supplied).unwrap_or_else(|| heap.clone());
        let cell = Self::holding_cell(&storage).ok_or_else(|| "TypeError: heap storage has no mutable cell".to_string())?;
        // Validation's temporary value must not share the row during writes.
        drop((supplied, storage));
        let extent = Self::heap_row_len(&cell);
        match verb {
            "heapify" => {
                for at in (0..extent / 2).rev() { self.heap_sink(&cell, at, maximum)?; }
                Ok(Value::Null)
            }
            "heappush" => {
                let item = item.ok_or_else(|| "TypeError: missing heap item".to_string())?;
                if let Value::Array(row) = &mut *cell.borrow_mut() { Rc::make_mut(row).push(item); }
                self.heap_climb(&cell, 0, extent, maximum)?;
                Ok(Value::Null)
            }
            "heappop" => {
                let last = match &mut *cell.borrow_mut() {
                    Value::Array(row) => Rc::make_mut(row).pop().ok_or_else(|| "IndexError: index out of range".to_string())?,
                    _ => return Err("TypeError: heap storage is not a list".into()),
                };
                if extent == 1 { return Ok(last); }
                let root = Self::heap_row_item(&cell, 0)?;
                if let Value::Array(row) = &mut *cell.borrow_mut() { Rc::make_mut(row)[0] = last; }
                self.heap_sink(&cell, 0, maximum)?;
                Ok(root)
            }
            "heapreplace" | "heappushpop" => {
                let item = item.ok_or_else(|| "TypeError: missing heap item".to_string())?;
                if verb == "heappushpop" {
                    if extent == 0 { return Ok(item); }
                    let root = Self::heap_row_item(&cell, 0)?;
                    let (a, b) = if maximum { (item.clone(), root) } else { (root, item.clone()) };
                    let comparison = self.special_dyad(&Action::Lt, &a, &b)?;
                    if !self.special_truth(&comparison)? { return Ok(item); }
                }
                let root = Self::heap_row_item(&cell, 0)?;
                if let Value::Array(row) = &mut *cell.borrow_mut() { Rc::make_mut(row)[0] = item; }
                self.heap_sink(&cell, 0, maximum)?;
                Ok(root)
            }
            _ => Err(format!("ValueError: unknown heap operation '{operation}'")),
        }
    }
}
