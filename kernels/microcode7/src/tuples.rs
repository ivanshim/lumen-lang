//! Sequence references and a bounded reserve of released Python tuple storage.
use std::cell::{Cell, RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use crate::data::Value;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static RESERVE: RefCell<Vec<Rc<Vec<Value>>>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone)]
pub struct Sequence {
    row: Rc<Vec<Value>>,
    tuple: bool,
    remembered: Rc<Cell<Option<i64>>>,
}
impl From<Rc<Vec<Value>>> for Sequence {
    fn from(row: Rc<Vec<Value>>) -> Self { Self { row, tuple: false, remembered: Rc::new(Cell::new(None)) } }
}
impl Deref for Sequence {
    type Target = Rc<Vec<Value>>;
    fn deref(&self) -> &Self::Target { &self.row }
}
impl DerefMut for Sequence {
    fn deref_mut(&mut self) -> &mut Self::Target { self.remembered = Rc::new(Cell::new(None)); &mut self.row }
}
impl Sequence {
    /// Retrieve the hash from a prior successful traversal of this tuple.
    pub fn known_hash(&self) -> Option<i64> { self.remembered.get() }
    /// Share a finished hash with references to the same immutable storage.
    pub fn record_hash(&self, number: i64) { self.remembered.set(Some(number)); }
    pub fn plain(values: Vec<Value>) -> Self { Rc::new(values).into() }

    pub fn tuple(values: Vec<Value>) -> Self {
        let tuple = ACTIVE.with(Cell::get);
        let available = if tuple {
            RESERVE.with(|reserve| {
                let mut reserve = reserve.borrow_mut();
                let capacity = reserve.iter().map(|row| row.capacity()).filter(|size| *size >= values.len()).min();
                capacity.and_then(|size| reserve.iter().rposition(|row| row.capacity() == size))
                    .map(|position| reserve.swap_remove(position))
            })
        } else { None };
        let row = match available {
            None => Rc::new(values),
            Some(mut row) => {
                Rc::get_mut(&mut row).expect("released tuple allocation").extend(values);
                row
            }
        };
        Self { row, tuple, remembered: Rc::new(Cell::new(None)) }
    }
}
impl Drop for Sequence {
    fn drop(&mut self) {
        if !self.tuple { return; }
        let Some(values) = Rc::get_mut(&mut self.row) else { return; };
        // Only the last real owner can reach here. Destroy all element
        // references before placing the empty allocation in the reserve.
        values.clear();
        let cost = values.capacity().saturating_mul(std::mem::size_of::<Value>());
        let _ = RESERVE.try_with(|reserve| {
            let mut reserve = reserve.borrow_mut();
            let occupied: usize = reserve.iter().map(|entry| entry.capacity().saturating_mul(std::mem::size_of::<Value>())).sum();
            if reserve.len() < 256 && cost <= (1024usize * 1024).saturating_sub(occupied) {
                reserve.push(self.row.clone());
            }
        });
    }
}

/// Tuple support in the language table selects this storage policy.
pub struct Scope { previous: bool }
impl Scope {
    pub fn enter(active: bool) -> Self { Self { previous: ACTIVE.with(|flag| flag.replace(active)) } }
}
impl Drop for Scope {
    fn drop(&mut self) {
        ACTIVE.with(|flag| flag.set(self.previous));
        if !self.previous { RESERVE.with(|reserve| reserve.borrow_mut().clear()); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Keep immutable aliases' hashes while discarding an altered row's memo.
    #[test]
    fn sequence_hash_does_not_follow_recycled_or_changed_contents() {
        let _scope = Scope::enter(true);
        let first = Sequence::tuple(vec![Value::Small(7)]);
        let mut next = first.clone();
        first.record_hash(29);
        assert_eq!(next.known_hash(), Some(29));
        Rc::make_mut(&mut next).clear();
        assert!(next.known_hash().is_none());
        assert_eq!(first.known_hash(), Some(29));
        drop(first);
        drop(next);
        let fresh = Sequence::tuple(vec![Value::Small(9)]);
        assert!(fresh.known_hash().is_none());
    }

    #[test]
    fn aliases_hold_elements_until_the_reserve_receives_empty_storage() {
        let _scope = Scope::enter(true);
        let payload = Rc::new(vec![Value::Small(7)]);
        let weak = Rc::downgrade(&payload);
        let first = Sequence::tuple(vec![Value::Vector(payload.clone().into())]);
        let allocation = Rc::as_ptr(&first);
        let buffer = first.as_ptr();
        let last = first.clone();
        assert_eq!(Rc::strong_count(&first), 2);
        drop(payload);
        drop(first);
        assert!(weak.upgrade().is_some());
        assert_eq!(Rc::strong_count(&last), 1);
        let live = Sequence::tuple(vec![Value::Small(9)]);
        assert_ne!(allocation, Rc::as_ptr(&live));
        assert!(matches!(&last[0], Value::Vector(_)));
        drop(live);
        drop(last);
        assert!(weak.upgrade().is_none());
        let reused = Sequence::tuple(vec![Value::Small(11)]);
        assert_eq!(allocation, Rc::as_ptr(&reused));
        assert_eq!(buffer, reused.as_ptr());
        assert_eq!(Rc::strong_count(&reused), 1);
        assert!(matches!(reused[0], Value::Small(11)));
    }
}
