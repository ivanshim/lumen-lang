//! A tally of the room a run takes.
//!
//! The host hands out its memory through [`Tally`], which is the system's
//! own allocator with a count kept beside it: every block handed out adds
//! its size to a running sum, and every block given back takes it away
//! again. What the sum holds is therefore the bytes alive at this moment,
//! not the bytes ever asked for.
//!
//! Two things are worth saying plainly about that number. It counts what
//! the whole process holds, since an allocator is one thing for the whole
//! of it, and it counts what was *asked* for rather than what the system
//! set aside: a request for three bytes is counted as three, though the
//! allocator beneath will have taken more. The reference implementation
//! counts the room of its own arena, which is a different number again,
//! near enough in size to be useful and not the same by the byte.
//!
//! A run is told where to count from with [`mark`]. Everything the host
//! held before that — the program's text, the language read in from
//! disk, the host's own workings — falls beneath the mark and is not laid
//! at the program's door. [`used`] answers with what has been taken since,
//! and [`most`] with the most that was ever taken at once.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// The bytes alive at this moment, counted for the whole process.
static LIVE: AtomicUsize = AtomicUsize::new(0);
/// The most that were ever alive at once since the mark was set.
static MOST: AtomicUsize = AtomicUsize::new(0);
/// Where the counting begins: what was alive when the mark was set.
static FROM: AtomicUsize = AtomicUsize::new(0);

/// The system's allocator, with a count of what it has handed out.
///
/// A host declares one of these as its `#[global_allocator]`; a host that
/// does not gets no count, and every reading below answers with nought.
pub struct Tally;

/// One more block of this size is alive; note it, and note a new high
/// water mark where there is one. The mark is read and written plainly
/// rather than swapped in a loop: two threads racing for it can lose a
/// byte or two of the highest reading, which is a fair price for keeping
/// an allocator out of a spin.
#[inline]
fn took(size: usize) {
    let now = LIVE.fetch_add(size, Relaxed) + size;
    if now > MOST.load(Relaxed) {
        MOST.store(now, Relaxed);
    }
}

#[inline]
fn gave_back(size: usize) {
    LIVE.fetch_sub(size, Relaxed);
}

unsafe impl GlobalAlloc for Tally {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let block = System.alloc(layout);
        if !block.is_null() {
            took(layout.size());
        }
        block
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let block = System.alloc_zeroed(layout);
        if !block.is_null() {
            took(layout.size());
        }
        block
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        System.dealloc(block, layout);
        gave_back(layout.size());
    }

    /// A block grown or shrunk in place: the old size goes, the new size
    /// comes, and where the system refuses the change nothing at all is
    /// counted, since the old block still stands.
    unsafe fn realloc(&self, block: *mut u8, layout: Layout, wanted: usize) -> *mut u8 {
        let moved = System.realloc(block, layout, wanted);
        if !moved.is_null() {
            gave_back(layout.size());
            took(wanted);
        }
        moved
    }
}

/// Begin the count here: what is alive now is the program's due, and
/// everything the host held before it is not.
pub fn mark() {
    let now = LIVE.load(Relaxed);
    FROM.store(now, Relaxed);
    MOST.store(now, Relaxed);
}

/// The bytes alive now that were not alive at the mark. A run that has
/// given back more than it took since the mark answers with nought
/// rather than with a number below it.
pub fn used() -> usize {
    LIVE.load(Relaxed).saturating_sub(FROM.load(Relaxed))
}

/// The most that were ever alive at once since the mark, counted the
/// same way.
pub fn most() -> usize {
    MOST.load(Relaxed).saturating_sub(FROM.load(Relaxed))
}

/// Forget the high water mark: the most ever alive is taken to be what
/// is alive at this moment, and the count of it begins again from there.
pub fn forget_most() {
    MOST.store(LIVE.load(Relaxed), Relaxed);
}
