#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
use alloc::{GlobalAlloc, Layout};
#[cfg(feature = "std")]
use std::alloc::{GlobalAlloc, Layout};

use core::cell::RefCell;

pub use alloc_counter_macro::no_alloc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AllocMode {
    Allow,
    Deny,
    Forbid,
}

thread_local!(static COUNTERS: RefCell<(usize, usize, usize)> = RefCell::new((0, 0, 0)));
thread_local!(static ALLOC_MODE: RefCell<AllocMode> = RefCell::new(AllocMode::Allow));

#[inline]
fn panicking() -> bool {
    #[cfg(feature = "std")]
    {
        std::thread::panicking()
    }

    #[cfg(not(feature = "std"))]
    false
}

pub struct AllocCounter<A>(pub A);

pub type AllocCounterSystem = AllocCounter<std::alloc::System>;
pub const ALLOC_COUNTER_SYSTEM: AllocCounterSystem = AllocCounter(std::alloc::System);

unsafe impl<A> GlobalAlloc for AllocCounter<A>
where
    A: GlobalAlloc,
{
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTERS.with(|counters| counters.borrow_mut().0 += 1);

        ALLOC_MODE.with(|mode| {
            if *mode.borrow() != AllocMode::Allow && !panicking() {
                panic!(
                    "Unexpected allocation of size {}, align {}.",
                    layout.size(),
                    layout.align(),
                );
            }
        });

        self.0.alloc(layout)
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        COUNTERS.with(|counters| counters.borrow_mut().1 += 1);

        ALLOC_MODE.with(|mode| {
            if *mode.borrow() != AllocMode::Allow && !panicking() {
                panic!(
                    "Unexpected reallocation of size {} -> {}, align {}.",
                    layout.size(),
                    new_size,
                    layout.align(),
                );
            }
        });

        self.0.realloc(ptr, layout, new_size)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        COUNTERS.with(|counters| counters.borrow_mut().2 += 1);

        ALLOC_MODE.with(|mode| {
            if *mode.borrow() != AllocMode::Allow && !panicking() {
                panic!(
                    "Unexpected deallocation of size {}, align {}.",
                    layout.size(),
                    layout.align(),
                );
            }
        });

        self.0.dealloc(ptr, layout)
    }
}

struct Guard(AllocMode);

impl Guard {
    fn new(mode: AllocMode) -> Self {
        ALLOC_MODE.with(|x| {
            let mut x = x.borrow_mut();
            let was = *x;
            *x = match was {
                AllocMode::Forbid => AllocMode::Forbid,
                _ => mode,
            };
            Guard(was)
        })
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        ALLOC_MODE.with(|x| *x.borrow_mut() = self.0);
    }
}

pub fn count_alloc<F, R>(f: F) -> ((usize, usize, usize), R)
where
    F: Fn() -> R,
{
    let (a, b, c) = COUNTERS.with(|counters| *counters.borrow());
    let r = f();
    let (d, e, f) = COUNTERS.with(|counters| *counters.borrow());

    ((d - a, e - b, f - c), r)
}

pub fn allow_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = Guard::new(AllocMode::Allow);
    f()
}

pub fn deny_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = Guard::new(AllocMode::Deny);
    f()
}

pub fn forbid_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = Guard::new(AllocMode::Forbid);
    f()
}
