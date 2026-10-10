//! Shared sequence ownership. Only dead Python tuple storage is recycled.
use std::cell::{Cell, RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use crate::value::Value;

const LIMIT: usize = 1024 * 1024;
const SLOTS: usize = 256;
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static EMPTY: Rc<Vec<Value>> = Rc::new(Vec::new());
    static FREE: RefCell<Released> = const { RefCell::new(Released { rows: Vec::new(), bytes: 0 }) };
}

// Budget accounting belongs to the pool, not to each released tuple.
struct Released { rows: Vec<Rc<Vec<Value>>>, bytes: usize }
impl Released {
    fn take(&mut self, needed: usize) -> Option<Rc<Vec<Value>>> {
        let mut best = None;
        let mut capacity = usize::MAX;
        for (at, row) in self.rows.iter().enumerate().rev() {
            let size = row.capacity();
            if size >= needed && size < capacity {
                best = Some(at); capacity = size;
                if size == needed { break; }
            }
        }
        best.map(|at| {
            let row = self.rows.swap_remove(at);
            self.bytes -= row.capacity().saturating_mul(std::mem::size_of::<Value>());
            row
        })
    }
    fn clear(&mut self) { self.rows.clear(); self.bytes = 0; }
}

/// An ordinary Rc for arrays, or the final-owner release boundary for tuples.
/// Iterator views clone this wrapper, so their references also delay release.
#[derive(Debug, Clone)]
pub struct Items {
    storage: Rc<Vec<Value>>,
    recycle: bool,
    hash: Rc<Cell<Option<i64>>>,
}
impl From<Rc<Vec<Value>>> for Items {
    fn from(storage: Rc<Vec<Value>>) -> Self { Self { storage, recycle: false, hash: Rc::new(Cell::new(None)) } }
}
impl Deref for Items {
    type Target = Rc<Vec<Value>>;
    fn deref(&self) -> &Self::Target { &self.storage }
}
impl DerefMut for Items {
    fn deref_mut(&mut self) -> &mut Self::Target { self.hash = Rc::new(Cell::new(None)); &mut self.storage }
}
impl Items {
    /// Read a completed Python tuple hash shared by all aliases.
    pub fn cached_hash(&self) -> Option<i64> { self.hash.get() }
    /// Retain only a hash whose element callbacks completed successfully.
    pub fn save_hash(&self, value: i64) { self.hash.set(Some(value)); }
    pub fn plain(values: Vec<Value>) -> Self { Rc::new(values).into() }

    pub fn tuple(parts: Vec<Value>) -> Self {
        let recycle = ENABLED.with(Cell::get);
        if recycle && parts.is_empty() {
            return Self { storage: EMPTY.with(Rc::clone), recycle: false, hash: Rc::new(Cell::new(None)) };
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
        Self { storage, recycle, hash: Rc::new(Cell::new(None)) }
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
            if free.rows.len() < SLOTS && bytes <= LIMIT.saturating_sub(free.bytes) {
                free.bytes += bytes;
                free.rows.push(self.storage.clone());
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
    // Preserve memoized hashes across aliases and reset them for new storage.
    #[test]
    fn tuple_hash_lives_with_its_immutable_row() {
        let _scope = Scope::enter(true);
        let original = Items::tuple(vec![Value::Small(1)]);
        original.save_hash(43);
        let mut alias = original.clone();
        assert_eq!(alias.cached_hash(), Some(43));
        Rc::make_mut(&mut alias).push(Value::Small(2));
        assert_eq!(alias.cached_hash(), None);
        assert_eq!(original.cached_hash(), Some(43));
        drop(alias);
        drop(original);
        assert_eq!(Items::tuple(vec![Value::Small(3)]).cached_hash(), None);
    }

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
