//! Shared sequence ownership. Only dead Python tuple storage is recycled.
use std::cell::{Cell, RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use crate::value::Value;

const LIMIT: usize = 1024 * 1024;
const SLOTS: usize = 256;
// Rows of up to this capacity are shelved by their exact capacity, so
// the usual small tuple finds a row without looking at the others.
const SHELVES: usize = 16;
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static EMPTY: Rc<Vec<Value>> = Rc::new(Vec::new());
    static FREE: RefCell<Released> = const { RefCell::new(Released { shelves: Vec::new(), large: Vec::new(), count: 0, bytes: 0 }) };
}

// Budget accounting belongs to the pool, not to each released tuple.
struct Released { shelves: Vec<Vec<Rc<Vec<Value>>>>, large: Vec<Rc<Vec<Value>>>, count: usize, bytes: usize }
impl Released {
    fn take(&mut self, needed: usize) -> Option<Rc<Vec<Value>>> {
        if self.count == 0 { return None; }
        let row = self.shelves.iter_mut().skip(needed).find_map(Vec::pop).or_else(|| {
            let (at, _) = self.large.iter().enumerate().rev()
                .filter(|(_, row)| row.capacity() >= needed)
                .min_by_key(|(_, row)| row.capacity())?;
            Some(self.large.swap_remove(at))
        })?;
        self.count -= 1;
        self.bytes -= row.capacity().saturating_mul(std::mem::size_of::<Value>());
        Some(row)
    }
    fn keep(&mut self, row: Rc<Vec<Value>>) {
        let size = row.capacity();
        if size < SHELVES {
            if self.shelves.is_empty() { self.shelves.resize_with(SHELVES, Vec::new); }
            self.shelves[size].push(row);
        } else { self.large.push(row); }
        self.count += 1;
    }
    fn clear(&mut self) { self.shelves.clear(); self.large.clear(); self.count = 0; self.bytes = 0; }
}

/// An ordinary Rc for arrays, or the final-owner release boundary for tuples.
/// Iterator views clone this wrapper, so their references also delay release.
#[derive(Debug, Clone)]
pub struct Items {
    storage: Rc<Vec<Value>>,
    recycle: bool,
}
impl From<Rc<Vec<Value>>> for Items {
    fn from(storage: Rc<Vec<Value>>) -> Self { Self { storage, recycle: false } }
}
impl Deref for Items {
    type Target = Rc<Vec<Value>>;
    fn deref(&self) -> &Self::Target { &self.storage }
}
impl DerefMut for Items {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.storage }
}
impl Items {
    pub fn plain(values: Vec<Value>) -> Self { Rc::new(values).into() }

    pub fn tuple(parts: Vec<Value>) -> Self {
        let recycle = ENABLED.with(Cell::get);
        if recycle && parts.is_empty() {
            return Self { storage: EMPTY.with(Rc::clone), recycle: false };
        }
        let stored = if recycle {
            FREE.with(|free| {
                let mut free = free.borrow_mut();
                free.take(parts.len())
            })
        } else { None };
        let storage = if let Some(mut storage) = stored {
            // The pool owns exactly one strong reference to an empty row.
            Rc::get_mut(&mut storage).expect("dead tuple storage").extend(parts);
            storage
        } else { Rc::new(parts) };
        Self { storage, recycle }
    }
}
impl Drop for Items {
    fn drop(&mut self) {
        if !self.recycle { return; }
        let Some(parts) = Rc::get_mut(&mut self.storage) else { return; };
        // Release elements before borrowing the pool: nested tuples can return
        // their own storage here. The pool never owns program references.
        parts.clear();
        let bytes = parts.capacity().saturating_mul(std::mem::size_of::<Value>());
        let _ = FREE.try_with(|free| {
            let mut free = free.borrow_mut();
            if free.count < SLOTS && bytes <= LIMIT.saturating_sub(free.bytes) {
                free.bytes += bytes;
                free.keep(self.storage.clone());
            }
        });
    }
}

/// The existing Python tuple label enables recycling for the entire run,
/// including compilation and imported modules. Nested runs restore the scope.
pub struct Scope(bool);
impl Scope {
    pub fn enter(enabled: bool) -> Self { Self(ENABLED.with(|old| old.replace(enabled))) }
}
impl Drop for Scope {
    fn drop(&mut self) {
        ENABLED.with(|enabled| enabled.set(self.0));
        if !self.0 { FREE.with(|free| free.borrow_mut().clear()); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_python_tuples_share_storage_without_aliasing_lists() {
        let _scope = Scope::enter(true);
        let first = Items::tuple(Vec::new());
        let second = Items::tuple(Vec::new());
        let array = Items::plain(Vec::new());
        assert_eq!(Rc::as_ptr(&first), Rc::as_ptr(&second));
        assert_ne!(Rc::as_ptr(&first), Rc::as_ptr(&array));
    }
    #[test]
    fn released_rows_are_taken_back_by_size() {
        let _scope = Scope::enter(true);
        let small = Items::tuple(vec![Value::Small(1), Value::Small(2)]);
        let large = Items::tuple((0..40).map(Value::Small).collect());
        let (small_at, large_at) = (Rc::as_ptr(&small), Rc::as_ptr(&large));
        drop(small);
        drop(large);
        // Each size finds the row of its own size, and a size beyond every
        // shelf the smallest large row that holds it.
        let two = Items::tuple(vec![Value::Small(3), Value::Small(4)]);
        let thirty_three = Items::tuple((0..33).map(Value::Small).collect());
        assert_eq!(Rc::as_ptr(&two), small_at);
        assert_eq!(Rc::as_ptr(&thirty_three), large_at);
        assert_eq!(thirty_three.len(), 33);
    }
    #[test]
    fn final_owner_releases_elements_and_recycles_actual_storage() {
        let _scope = Scope::enter(true);
        let payload = Rc::new(vec![Value::Small(7)]);
        let weak = Rc::downgrade(&payload);
        let tuple = Items::tuple(vec![Value::Array(payload.clone().into())]);
        let allocation = Rc::as_ptr(&tuple);
        let buffer = tuple.as_ptr();
        let alias = tuple.clone();
        assert_eq!(Rc::strong_count(&tuple), 2);
        drop(payload);
        drop(tuple);
        assert!(weak.upgrade().is_some());
        assert_eq!(Rc::strong_count(&alias), 1);
        let other = Items::tuple(vec![Value::Small(9)]);
        assert_ne!(allocation, Rc::as_ptr(&other));
        assert!(matches!(&alias[0], Value::Array(_)));
        drop(other);
        drop(alias);
        assert!(weak.upgrade().is_none());
        let next = Items::tuple(vec![Value::Small(11)]);
        assert_eq!(allocation, Rc::as_ptr(&next));
        assert_eq!(buffer, next.as_ptr());
        assert_eq!(Rc::strong_count(&next), 1);
        assert!(matches!(next[0], Value::Small(11)));
    }
}
