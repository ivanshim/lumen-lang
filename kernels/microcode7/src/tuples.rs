//! Sequence references and a bounded reserve of released Python tuple storage.
use std::cell::{Cell, RefCell};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use crate::data::Value;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static VACANT: Rc<Vec<Value>> = Rc::new(vec![]);
    static RESERVE: RefCell<TupleReserve> = const { RefCell::new(TupleReserve { allocations: Vec::new(), bins: [const { Vec::new() }; 64], occupied: 0, count: 0, retained_bytes: 0 }) };
}

// Budget accounting belongs to the pool, not to each released tuple.
struct TupleReserve { allocations: Vec<Rc<Vec<Value>>>, bins: [Vec<Rc<Vec<Value>>>; 64], occupied: u64, count: usize, retained_bytes: usize }
impl TupleReserve {
    fn acquire(&mut self, needed: usize) -> Option<Rc<Vec<Value>>> {
        if self.count == 0 { return None; }
        let candidates = if needed < 64 { self.occupied >> needed } else { 0 };
        if candidates != 0 {
            let bin = needed + candidates.trailing_zeros() as usize;
            let result = self.bins[bin].pop().unwrap();
            if self.bins[bin].is_empty() { self.occupied ^= 1u64 << bin; }
            self.count -= 1;
            self.retained_bytes -= result.capacity().saturating_mul(std::mem::size_of::<Value>());
            return Some(result);
        }
        let mut best = None;
        let mut capacity = usize::MAX;
        let mut cursor = self.allocations.len();
        while cursor > 0 {
            cursor -= 1;
            let available = self.allocations[cursor].capacity();
            if available < needed || available >= capacity { continue; }
            capacity = available;
            best = Some(cursor);
            if available == needed { break; }
        }
        best.map(|at| {
            let row = self.allocations.swap_remove(at);
            self.count -= 1;
            self.retained_bytes -= row.capacity().saturating_mul(std::mem::size_of::<Value>());
            row
        })
    }
    fn release(&mut self, row: Rc<Vec<Value>>) {
        self.count += 1;
        let bin = row.capacity();
        if bin >= 64 { self.allocations.push(row); }
        else {
            self.bins[bin].push(row);
            self.occupied |= 1u64 << bin;
        }
    }
    fn clear(&mut self) {
        for bucket in &mut self.bins { bucket.clear(); }
        self.allocations.clear(); self.occupied = 0; self.count = 0; self.retained_bytes = 0;
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
            if reserve.count < 256 && cost <= (1024usize * 1024).saturating_sub(reserve.retained_bytes) {
                reserve.retained_bytes += cost;
                reserve.release(self.row.clone());
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
