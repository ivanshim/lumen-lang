//! Who may touch the interpreter's shared storage. Each interpreter has
//! one lock, taken by the thread that runs it; a guarded cell hands out
//! its contents only to the thread that holds the lock of the interpreter
//! it was made for, and the lock cannot be given up while a borrow of any
//! such cell is alive. Nothing switches threads yet: the one thread that
//! runs a program takes the lock at entry and keeps it to the end.
#![allow(dead_code)] // the cells are taken into use by the next jobs

use std::cell::{Cell, UnsafeCell};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

static INTERPRETERS: AtomicU64 = AtomicU64::new(1);
static THREADS: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// This thread's token, never zero and never reused.
    static TOKEN: u64 = THREADS.fetch_add(1, Ordering::Relaxed);
    /// The interpreter whose lock this thread holds, 0 for none.
    static HOLDING: Cell<u64> = const { Cell::new(0) };
}

fn token() -> u64 { TOKEN.with(|t| *t) }

/// Why an access or a release was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refused {
    /// The thread holds no interpreter lock at all.
    NoOwner,
    /// The thread holds the lock of another interpreter.
    OtherInterpreter,
    /// The lock is already held, by this thread or another.
    Taken,
    /// The thread does not hold this lock.
    NotHeld,
    /// A borrow of guarded storage is still alive.
    Borrowed,
    /// The cell is lent out in a way that clashes with the request.
    Clash,
}

pub(crate) struct Gil {
    id: u64,
    owner: AtomicU64,
    live: AtomicUsize,
    /// The interpreter this thread was running when it took this lock (a nested run), 0 for none.
    under: AtomicU64,
}

impl Gil {
    pub(crate) fn new() -> Arc<Gil> {
        Arc::new(Gil { id: INTERPRETERS.fetch_add(1, Ordering::Relaxed), owner: AtomicU64::new(0), live: AtomicUsize::new(0), under: AtomicU64::new(0) })
    }

    /// Take the lock for the calling thread. A thread already running another interpreter may take this one for a nested run.
    pub(crate) fn acquire(self: &Arc<Gil>) -> Result<Hold, Refused> {
        let outer = HOLDING.with(|h| h.get());
        if outer == self.id { return Err(Refused::Taken); }
        self.owner.compare_exchange(0, token(), Ordering::Acquire, Ordering::Relaxed).map_err(|_| Refused::Taken)?;
        self.under.store(outer, Ordering::Relaxed);
        HOLDING.with(|h| h.set(self.id));
        Ok(Hold { gil: self.clone(), active: true, thread_bound: PhantomData })
    }

    /// Give the lock up; refused while any borrow is alive.
    fn unlock(&self) -> Result<(), Refused> {
        if HOLDING.with(|h| h.get()) != self.id || self.owner.load(Ordering::Relaxed) != token() { return Err(Refused::NotHeld); }
        if self.live.load(Ordering::Relaxed) != 0 { return Err(Refused::Borrowed); }
        HOLDING.with(|h| h.set(self.under.load(Ordering::Relaxed)));
        self.owner.store(0, Ordering::Release);
        Ok(())
    }

    /// Whether the calling thread may touch storage of this interpreter.
    pub(crate) fn check(&self) -> Result<(), Refused> {
        match HOLDING.with(|h| h.get()) {
            0 => Err(Refused::NoOwner),
            id if id != self.id => Err(Refused::OtherInterpreter),
            _ if self.owner.load(Ordering::Acquire) != token() => Err(Refused::NotHeld),
            _ => Ok(()),
        }
    }
}

/// The lock held by the running thread; dropping it gives the lock up.
pub(crate) struct Hold { gil: Arc<Gil>, active: bool, thread_bound: PhantomData<Rc<()>> }

impl Hold {
    pub(crate) fn gil(&self) -> &Arc<Gil> { &self.gil }

    pub(crate) fn release(&mut self) -> Result<(), Refused> {
        if !self.active { return Err(Refused::NotHeld); }
        self.gil.unlock()?;
        self.active = false;
        Ok(())
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        match self.release() {
            Err(Refused::Borrowed) if !std::thread::panicking() => panic!("interpreter lock dropped with a live borrow"),
            Err(_) | Ok(()) => {}
        }
    }
}

/// A cell whose contents may be reached only under its interpreter's lock.
pub(crate) struct Guarded<T> {
    gil: Arc<Gil>,
    /// Shared borrows alive, or -1 while lent out for writing.
    lent: Cell<isize>,
    inner: UnsafeCell<T>,
}

impl<T> Guarded<T> {
    pub(crate) fn new(gil: &Arc<Gil>, value: T) -> Self {
        Guarded { gil: gil.clone(), lent: Cell::new(0), inner: UnsafeCell::new(value) }
    }

    pub(crate) fn try_borrow(&self) -> Result<Shared<'_, T>, Refused> {
        self.gil.check()?;
        if self.lent.get() < 0 { return Err(Refused::Clash); }
        self.lent.set(self.lent.get() + 1);
        self.gil.live.fetch_add(1, Ordering::Relaxed);
        Ok(Shared { cell: self })
    }

    pub(crate) fn try_borrow_mut(&self) -> Result<Unique<'_, T>, Refused> {
        self.gil.check()?;
        if self.lent.get() != 0 { return Err(Refused::Clash); }
        self.lent.set(-1);
        self.gil.live.fetch_add(1, Ordering::Relaxed);
        Ok(Unique { cell: self })
    }

    pub(crate) fn borrow(&self) -> Shared<'_, T> {
        self.try_borrow().unwrap_or_else(|why| panic!("guarded cell refused a borrow: {why:?}"))
    }

    pub(crate) fn borrow_mut(&self) -> Unique<'_, T> {
        self.try_borrow_mut().unwrap_or_else(|why| panic!("guarded cell refused a mutable borrow: {why:?}"))
    }
}

pub(crate) struct Shared<'a, T> { cell: &'a Guarded<T> }
pub(crate) struct Unique<'a, T> { cell: &'a Guarded<T> }

impl<T> Deref for Shared<'_, T> {
    type Target = T;
    // SAFETY: the lock was checked when the borrow began and cannot be given up while it lives; no writer exists while `lent` is positive.
    fn deref(&self) -> &T { unsafe { &*self.cell.inner.get() } }
}

impl<T> Deref for Unique<'_, T> {
    type Target = T;
    // SAFETY: as for a shared borrow, and this is the only borrow.
    fn deref(&self) -> &T { unsafe { &*self.cell.inner.get() } }
}

impl<T> DerefMut for Unique<'_, T> {
    // SAFETY: `lent` is -1, so no other borrow of this cell exists.
    fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.cell.inner.get() } }
}

impl<T> Drop for Shared<'_, T> {
    fn drop(&mut self) {
        self.cell.lent.set(self.cell.lent.get() - 1);
        self.cell.gil.live.fetch_sub(1, Ordering::Relaxed);
    }
}

impl<T> Drop for Unique<'_, T> {
    fn drop(&mut self) {
        self.cell.lent.set(0);
        self.cell.gil.live.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_owner_is_refused() {
        let gil = Gil::new();
        let cell = Guarded::new(&gil, 1);
        assert_eq!(cell.try_borrow().err(), Some(Refused::NoOwner));
    }

    #[test]
    fn other_interpreter_is_refused() {
        let (mine, theirs) = (Gil::new(), Gil::new());
        let cell = Guarded::new(&theirs, 1);
        let _hold = mine.acquire().unwrap();
        assert_eq!(cell.try_borrow_mut().err(), Some(Refused::OtherInterpreter));
    }

    #[test]
    fn nested_borrows_follow_the_cell_rules() {
        let gil = Gil::new();
        let _hold = gil.acquire().unwrap();
        let cell = Guarded::new(&gil, vec![1]);
        let a = cell.borrow();
        let b = cell.borrow();
        assert_eq!(cell.try_borrow_mut().err(), Some(Refused::Clash));
        drop((a, b));
        let mut w = cell.borrow_mut();
        w.push(2);
        assert_eq!(cell.try_borrow().err(), Some(Refused::Clash));
        drop(w);
        assert_eq!(cell.borrow().len(), 2);
    }

    #[test]
    fn release_refused_with_a_live_borrow() {
        let gil = Gil::new();
        let mut hold = gil.acquire().unwrap();
        let cell = Guarded::new(&gil, 0);
        let b = cell.borrow();
        assert_eq!(hold.release(), Err(Refused::Borrowed));
        drop(b);
        assert_eq!(hold.release(), Ok(()));
        assert_eq!(cell.try_borrow().err(), Some(Refused::NoOwner));
        drop(hold);
        assert!(gil.acquire().is_ok());
    }

    #[test]
    fn a_released_guard_cannot_unlock_a_new_acquisition() {
        let gil = Gil::new();
        let mut old = gil.acquire().unwrap();
        old.release().unwrap();
        let current = gil.acquire().unwrap();
        assert_eq!(old.release(), Err(Refused::NotHeld));
        drop(old);
        assert_eq!(gil.check(), Ok(()));
        drop(current);
        assert_eq!(gil.check(), Err(Refused::NoOwner));
    }

    #[test]
    fn only_the_lock_is_transferable() {
        fn shareable<T: Send + Sync>() {}
        shareable::<Gil>();
        // Inference becomes ambiguous if Hold ever gains Send or Sync.
        trait AmbiguousSend<A> { fn verify() {} }
        impl<T: ?Sized> AmbiguousSend<()> for T {}
        impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
        let _ = <Hold as AmbiguousSend<_>>::verify;
        trait AmbiguousSync<A> { fn verify() {} }
        impl<T: ?Sized> AmbiguousSync<()> for T {}
        impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
        let _ = <Hold as AmbiguousSync<_>>::verify;
    }

    #[test]
    fn second_acquire_and_other_thread_are_refused() {
        let gil = Gil::new();
        let _hold = gil.acquire().unwrap();
        assert_eq!(gil.acquire().err(), Some(Refused::Taken));
        let g = gil.clone();
        let cell = Guarded::new(&gil, 0);
        let seen = std::thread::scope(|s| s.spawn(|| (g.acquire().err(), g.check().err())).join().unwrap());
        assert_eq!(seen, (Some(Refused::Taken), Some(Refused::NoOwner)));
        let _ = cell;
    }

    #[test]
    fn a_nested_run_hands_back_to_the_outer_interpreter() {
        let (outer, inner) = (Gil::new(), Gil::new());
        let cell = Guarded::new(&outer, 1);
        let _o = outer.acquire().unwrap();
        {
            let _i = inner.acquire().unwrap();
            assert_eq!(cell.try_borrow().err(), Some(Refused::OtherInterpreter));
        }
        assert_eq!(*cell.borrow(), 1);
    }

    #[test]
    fn unwinding_drops_the_borrow_and_the_lock() {
        let gil = Gil::new();
        let g = gil.clone();
        let outcome = std::thread::spawn(move || {
            let _hold = g.acquire().unwrap();
            let cell = Guarded::new(&g, 0);
            let _w = cell.borrow_mut();
            panic!("boom");
        }).join();
        assert!(outcome.is_err());
        assert_eq!(gil.live.load(Ordering::Relaxed), 0);
        assert!(gil.acquire().is_ok());
    }
}
