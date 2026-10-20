//! Sequence references and a bounded reserve of released Python tuple storage.
use std::cell::{Cell, RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use crate::data::Value;

// Released rows of up to this many slots are kept in a tier of their own size.
const TIERS: usize = 16;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static VACANT: Rc<Vec<Value>> = Rc::new(vec![]);
    static RESERVE: RefCell<TupleReserve> = const { RefCell::new(TupleReserve { tiers: [const { Vec::new() }; TIERS], oversize: Vec::new(), stored: 0, retained_bytes: 0 }) };
}

// Budget accounting belongs to the pool, not to each released tuple.
struct TupleReserve { tiers: [Vec<Rc<Vec<Value>>>; TIERS], oversize: Vec<Rc<Vec<Value>>>, stored: usize, retained_bytes: usize }
impl TupleReserve {
    fn acquire(&mut self, needed: usize) -> Option<Rc<Vec<Value>>> {
        if self.stored == 0 { return None; }
        let mut found = None;
        let mut tier = needed;
        while found.is_none() && tier < TIERS {
            found = self.tiers[tier].pop();
            tier += 1;
        }
        if found.is_none() {
            let mut best: Option<(usize, usize)> = None;
            for (at, row) in self.oversize.iter().enumerate() {
                let room = row.capacity();
                if room >= needed && best.map_or(true, |(_, least)| room < least) { best = Some((at, room)); }
            }
            found = best.map(|(at, _)| self.oversize.swap_remove(at));
        }
        let row = found?;
        self.stored -= 1;
        self.retained_bytes -= row.capacity().saturating_mul(std::mem::size_of::<Value>());
        Some(row)
    }
    fn shelve(&mut self, row: Rc<Vec<Value>>) {
        let room = row.capacity();
        if room < TIERS { self.tiers[room].push(row); } else { self.oversize.push(row); }
        self.stored += 1;
    }
    fn clear(&mut self) {
        for tier in self.tiers.iter_mut() { tier.clear(); }
        self.oversize.clear();
        self.stored = 0;
        self.retained_bytes = 0;
    }
}

#[derive(Debug, Clone)]
pub struct Sequence {
    row: Rc<Vec<Value>>,
    tuple: bool,
}
impl From<Rc<Vec<Value>>> for Sequence {
    fn from(row: Rc<Vec<Value>>) -> Self { Self { row, tuple: false } }
}
impl Deref for Sequence {
    type Target = Rc<Vec<Value>>;
    fn deref(&self) -> &Self::Target { &self.row }
}
impl DerefMut for Sequence {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.row }
}
impl Sequence {
    pub fn plain(values: Vec<Value>) -> Self { Rc::new(values).into() }

    pub fn tuple(values: Vec<Value>) -> Self {
        let tuple = ACTIVE.with(Cell::get);
        if values.is_empty() && tuple {
            let row = VACANT.with(|saved| saved.clone());
            return Self { row, tuple: false };
        }
        let available = if tuple {
            RESERVE.with(|reserve| {
                let mut reserve = reserve.borrow_mut();
                reserve.acquire(values.len())
            })
        } else { None };
        let row = match available {
            None => Rc::new(values),
            Some(mut row) => {
                Rc::get_mut(&mut row).expect("released tuple allocation").extend(values);
                row
            }
        };
        Self { row, tuple }
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
            if reserve.stored < 256 && cost <= (1024usize * 1024).saturating_sub(reserve.retained_bytes) {
                reserve.retained_bytes += cost;
                reserve.shelve(self.row.clone());
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
    #[test]
    fn vacant_tuples_keep_one_allocation_and_vectors_keep_their_own() {
        let _policy = Scope::enter(true);
        let tuples = [Sequence::tuple(vec![]), Sequence::tuple(vec![])];
        let vector = Sequence::plain(vec![]);
        assert!(Rc::ptr_eq(&tuples[0], &tuples[1]));
        assert!(!Rc::ptr_eq(&tuples[1], &vector));
    }
    #[test]
    fn released_rows_come_back_by_size() {
        let _scope = Scope::enter(true);
        let pair = Sequence::tuple(vec![Value::Small(1), Value::Small(2)]);
        let long = Sequence::tuple((0..40).map(Value::Small).collect());
        let (pair_row, long_row) = (Rc::as_ptr(&pair), Rc::as_ptr(&long));
        drop(pair);
        drop(long);
        let again = Sequence::tuple(vec![Value::Small(3), Value::Small(4)]);
        let longer = Sequence::tuple((0..33).map(Value::Small).collect());
        assert_eq!(Rc::as_ptr(&again), pair_row);
        assert_eq!(Rc::as_ptr(&longer), long_row);
        assert_eq!(longer.len(), 33);
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
