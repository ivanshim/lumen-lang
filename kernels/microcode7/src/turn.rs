// Whose turn it is to touch the interpreter's shared storage. Every
// interpreter has one Turn, held by the thread that is running it; a
// Pinned cell lets its contents be looked at or changed only by the
// thread holding the Turn of the interpreter the cell was made for, and a
// Turn cannot be handed back while anything is still looking at a cell.
// No thread changes over yet: the thread that starts a program holds its
// Turn from entry to the end.
#![allow(dead_code)] // the cells are taken into use by the next jobs

use std::cell::{Cell as Slot, UnsafeCell};
use std::ops::{Deref, DerefMut};
use std::{marker::PhantomData, rc::Rc};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::*};
use std::sync::Arc;

static NEXT_SERIAL: AtomicU64 = AtomicU64::new(1);
static NEXT_RUNNER: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static RUNNER: u64 = NEXT_RUNNER.fetch_add(1, Relaxed);
    static STANDING_IN: Slot<u64> = const { Slot::new(0) };
}

fn runner() -> u64 { RUNNER.with(|r| *r) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    Nobody,
    Elsewhere,
    Occupied,
    NotYours,
    Looked,
    Busy,
}

pub(crate) struct Turn {
    serial: u64,
    holder: AtomicU64,
    pins: AtomicUsize,
    beneath: AtomicU64, // serial of the Turn this runner stood in when it took this one
}

impl Turn {
    pub(crate) fn fresh() -> Arc<Turn> {
        let serial = NEXT_SERIAL.fetch_add(1, Relaxed);
        Arc::new(Turn { serial, holder: AtomicU64::new(0), pins: AtomicUsize::new(0), beneath: AtomicU64::new(0) })
    }

    pub(crate) fn take(self: &Arc<Turn>) -> Result<Taken, Refusal> {
        let below = STANDING_IN.with(Slot::get);
        if below == self.serial { return Err(Refusal::Occupied); }
        if self.holder.compare_exchange(0, runner(), Acquire, Relaxed).is_err() { return Err(Refusal::Occupied); }
        self.beneath.store(below, Relaxed);
        STANDING_IN.with(|s| s.set(self.serial));
        Ok(Taken { turn: Arc::clone(self), returned: false, local: PhantomData })
    }

    fn return_to_caller(&self) -> Result<(), Refusal> {
        let mine = STANDING_IN.with(Slot::get) == self.serial && self.holder.load(Relaxed) == runner();
        if !mine { return Err(Refusal::NotYours); }
        if self.pins.load(Relaxed) > 0 { return Err(Refusal::Looked); }
        STANDING_IN.with(|s| s.set(self.beneath.load(Relaxed)));
        self.holder.store(0, Release);
        Ok(())
    }

    pub(crate) fn admit(&self) -> Result<(), Refusal> {
        let standing = STANDING_IN.with(Slot::get);
        if standing == 0 { return Err(Refusal::Nobody); }
        if standing != self.serial { return Err(Refusal::Elsewhere); }
        if self.holder.load(Acquire) == runner() { Ok(()) } else { Err(Refusal::NotYours) }
    }
}

/// Proof that the running thread has the Turn; dropping it hands the Turn back.
pub(crate) struct Taken {
    turn: Arc<Turn>,
    returned: bool,
    local: PhantomData<Rc<()>>,
}

impl Taken {
    pub(crate) fn turn(&self) -> &Arc<Turn> { &self.turn }

    pub(crate) fn hand_back(&mut self) -> Result<(), Refusal> {
        if self.returned { return Err(Refusal::NotYours); }
        match self.turn.return_to_caller() {
            Ok(()) => { self.returned = true; Ok(()) }
            Err(reason) => Err(reason),
        }
    }
}

impl Drop for Taken {
    fn drop(&mut self) {
        if let Err(Refusal::Looked) = self.hand_back() {
            if !std::thread::panicking() { panic!("turn handed back while a cell is still pinned"); }
        }
    }
}

/// Storage that answers only to the holder of one interpreter's Turn.
pub(crate) struct Pinned<T> {
    turn: Arc<Turn>,
    looks: Slot<isize>, // how many look at it; -1 while one changes it
    stuff: UnsafeCell<T>,
}

impl<T> Pinned<T> {
    pub(crate) fn within(turn: &Arc<Turn>, stuff: T) -> Self {
        Pinned { turn: Arc::clone(turn), looks: Slot::new(0), stuff: UnsafeCell::new(stuff) }
    }

    fn pin(&self, change: bool) -> Result<(), Refusal> {
        self.turn.admit()?;
        let now = self.looks.get();
        if now < 0 || (change && now > 0) { return Err(Refusal::Busy); }
        self.looks.set(if change { -1 } else { now + 1 });
        self.turn.pins.fetch_add(1, Relaxed);
        Ok(())
    }

    pub(crate) fn look(&self) -> Result<Look<'_, T>, Refusal> { self.pin(false).map(|_| Look(self)) }
    pub(crate) fn edit(&self) -> Result<Edit<'_, T>, Refusal> { self.pin(true).map(|_| Edit(self)) }

    pub(crate) fn look_or_die(&self) -> Look<'_, T> {
        match self.look() { Ok(l) => l, Err(r) => panic!("pinned cell would not be looked at: {r:?}") }
    }

    pub(crate) fn edit_or_die(&self) -> Edit<'_, T> {
        match self.edit() { Ok(e) => e, Err(r) => panic!("pinned cell would not be edited: {r:?}") }
    }
}

pub(crate) struct Look<'a, T>(&'a Pinned<T>);
pub(crate) struct Edit<'a, T>(&'a Pinned<T>);

impl<T> Deref for Look<'_, T> {
    type Target = T;
    // SAFETY: admitted when pinned, and the Turn stays held while pinned; nobody edits while looks are counted.
    fn deref(&self) -> &T { unsafe { &*self.0.stuff.get() } }
}

impl<T> Deref for Edit<'_, T> {
    type Target = T;
    // SAFETY: admitted when pinned; this is the only pin on the cell.
    fn deref(&self) -> &T { unsafe { &*self.0.stuff.get() } }
}

impl<T> DerefMut for Edit<'_, T> {
    // SAFETY: the count reads -1, so no other pin on the cell exists.
    fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.0.stuff.get() } }
}

impl<T> Drop for Look<'_, T> {
    fn drop(&mut self) {
        self.0.looks.set(self.0.looks.get() - 1);
        self.0.turn.pins.fetch_sub(1, Relaxed);
    }
}

impl<T> Drop for Edit<'_, T> {
    fn drop(&mut self) {
        self.0.looks.set(0);
        self.0.turn.pins.fetch_sub(1, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nobody_standing_is_refused() {
        let turn = Turn::fresh();
        assert_eq!(Pinned::within(&turn, 5).look().err(), Some(Refusal::Nobody));
    }

    #[test]
    fn a_cell_of_another_interpreter_is_refused() {
        let (here, there) = (Turn::fresh(), Turn::fresh());
        let cell = Pinned::within(&there, ());
        let _t = here.take().unwrap();
        assert_eq!(cell.edit().err(), Some(Refusal::Elsewhere));
    }

    #[test]
    fn looks_and_edits_exclude_each_other() {
        let turn = Turn::fresh();
        let _t = turn.take().unwrap();
        let cell = Pinned::within(&turn, String::new());
        let (a, b) = (cell.look_or_die(), cell.look_or_die());
        assert_eq!(cell.edit().err(), Some(Refusal::Busy));
        drop((a, b));
        let mut e = cell.edit_or_die();
        e.push('x');
        assert_eq!(cell.look().err(), Some(Refusal::Busy));
        drop(e);
        assert_eq!(&*cell.look_or_die(), "x");
    }

    #[test]
    fn handing_back_waits_for_the_pins() {
        let turn = Turn::fresh();
        let mut t = turn.take().unwrap();
        let cell = Pinned::within(&turn, 1u8);
        let l = cell.look_or_die();
        assert_eq!(t.hand_back(), Err(Refusal::Looked));
        drop(l);
        assert_eq!(t.hand_back(), Ok(()));
        assert_eq!(cell.look().err(), Some(Refusal::Nobody));
        drop(t);
        assert!(turn.take().is_ok());
    }

    #[test]
    fn an_old_receipt_does_not_return_the_next_turn() {
        let turn = Turn::fresh();
        let mut receipt = turn.take().unwrap();
        assert!(receipt.hand_back().is_ok());
        let next = turn.take().unwrap();
        assert_eq!(receipt.hand_back(), Err(Refusal::NotYours));
        drop(receipt);
        assert!(turn.admit().is_ok());
        drop(next);
        assert_eq!(turn.admit(), Err(Refusal::Nobody));
    }

    #[test]
    fn receipts_stay_local_but_turns_can_be_shared() {
        fn transferable<T: Sync + Send>() {}
        transferable::<Turn>();
        // Overlapping implementations reject a transferable or shareable receipt.
        trait NotTransferable<Kind> { fn check() {} }
        impl<T> NotTransferable<()> for T {}
        impl<T: Send> NotTransferable<bool> for T {}
        let _send = <Taken as NotTransferable<_>>::check;
        trait NotShareable<Kind> { fn check() {} }
        impl<T> NotShareable<()> for T {}
        impl<T: Sync> NotShareable<bool> for T {}
        let _sync = <Taken as NotShareable<_>>::check;
    }

    #[test]
    fn a_second_take_and_another_thread_are_refused() {
        let turn = Turn::fresh();
        let _t = turn.take().unwrap();
        assert_eq!(turn.take().err(), Some(Refusal::Occupied));
        let other = Arc::clone(&turn);
        let seen = std::thread::spawn(move || (other.take().err(), other.admit().err())).join().unwrap();
        assert_eq!(seen, (Some(Refusal::Occupied), Some(Refusal::Nobody)));
    }

    #[test]
    fn a_nested_interpreter_gives_the_turn_back_to_the_outer_one() {
        let (outer, inner) = (Turn::fresh(), Turn::fresh());
        let cell = Pinned::within(&outer, 7);
        let _o = outer.take().unwrap();
        {
            let _i = inner.take().unwrap();
            assert_eq!(cell.look().err(), Some(Refusal::Elsewhere));
        }
        assert_eq!(*cell.look_or_die(), 7);
    }

    #[test]
    fn unwinding_lets_go_of_pin_and_turn() {
        let turn = Turn::fresh();
        let inner = Arc::clone(&turn);
        let result = std::thread::spawn(move || {
            let _t = inner.take().unwrap();
            let cell = Pinned::within(&inner, 0);
            let _e = cell.edit_or_die();
            panic!("unwind");
        }).join();
        assert!(result.is_err());
        assert_eq!(turn.pins.load(Relaxed), 0);
        assert!(turn.take().is_ok());
    }
}
