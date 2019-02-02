#![cfg_attr(not(feature = "std"), feature(alloc, allocator_api))]

//! # Alloc counters
//!
//! A redesign of the [Quick and Dirty Allocation Profiling
//! Tool](https://github.com/bspeice/qadapt).
//!
//!
//! ## Features
//!
//! * Count allocations, reallocations and deallocations individually with `count_alloc`.
//!
//! * Allow, deny, and forbid use of the global allocator with `allow_alloc`, `deny_alloc` and
//! `forbid_alloc`.
//!
//! * `#[no_alloc]` function attribute to deny and `#[no_alloc(forbid)]` to forbid use of the
//! global allocator.
//!
//!
//! ## Limitations and known issues
//!
//! * Methods must either take a reference to `self` or `Self` must be a `Copy` type.
//!
//!
//! ## Usage
//!
//! An `AllocCounter<A>` wraps an allocator `A` to individually count the number of calls to
//! `alloc`, `realloc`, and `dealloc`.
//!
//! ```rust
//! # type MyAllocator = std::alloc::System;
//! # const MyAllocator: MyAllocator = std::alloc::System;
//! # fn main() {}
//!
//! use alloc_counter::AllocCounter;
//!
//! #[global_allocator]
//! static A: AllocCounter<MyAllocator> = AllocCounter(MyAllocator);
//! ```
//!
//! Std-users may prefer to inherit their system's allocator.
//!
//! ```rust
//! # fn main() {}
//! use alloc_counter::AllocCounterSystem;
//!
//! #[global_allocator]
//! static A: AllocCounterSystem = AllocCounterSystem;
//! ```
//!
//! To count the allocations of an expression, use `count_alloc`.
//!
//! ```rust
//! # use alloc_counter::{AllocCounterSystem, count_alloc};
//!
//! # #[global_allocator]
//! # static A: AllocCounterSystem = AllocCounterSystem;
//!
//! # fn main() {
//! assert_eq!(
//!     count_alloc(|| {
//!         // no alloc
//!         let mut v = Vec::new();
//!         // alloc
//!         v.push(0);
//!         // realloc
//!         v.push(1);
//!         // dealloc
//!     })
//!     .0,
//!     (1, 1, 1)
//! );
//! # }
//! ```
//!
//! To deny allocations for an expression use `deny_alloc`.
//!
//! ```rust,should_panic
//! # use alloc_counter::{AllocCounterSystem, deny_alloc};
//! # #[global_allocator]
//! # static A: AllocCounterSystem = AllocCounterSystem;
//! # fn main() {
//! fn foo(b: Box<i32>) {
//!     deny_alloc(|| drop(b))
//! }
//! foo(Box::new(0));
//! # }
//! ```
//!
//! Similar to Rust's lints if you deny something, you can still allow it. Unless you also forbid
//! it. However, forbidding and then allowing is valid to express, the forbid behavior will be
//! applied.
//!
//! ```rust,should_panic
//! # use alloc_counter::{AllocCounterSystem, forbid_alloc, allow_alloc};
//! # #[global_allocator]
//! # static A: AllocCounterSystem = AllocCounterSystem;
//! # fn main() {
//! fn foo(b: Box<i32>) {
//!     forbid_alloc(|| allow_alloc(|| drop(b)))
//! }
//! foo(Box::new(0));
//! # }
//! ```
//!
//! For added sugar you may use the `#[no_alloc]` attribute on functions, including methods with
//! self-binds. `#[no_alloc]` expands to calling `deny_alloc` and forcefully moves the parameters
//! into the checked block. `#[no_alloc(forbid)]` calls `forbid_alloc`.
//!
//! ```rust,should_panic
//! # use alloc_counter::{AllocCounterSystem, allow_alloc, no_alloc};
//! # #[global_allocator]
//! # static A: AllocCounterSystem = AllocCounterSystem;
//! # fn main() {
//! #[no_alloc(forbid)]
//! fn foo(b: Box<i32>) {
//!     allow_alloc(|| drop(b))
//! }
//! foo(Box::new(0));
//! # }
//! ```

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::alloc::{GlobalAlloc, Layout};
#[cfg(feature = "std")]
use std::alloc::{GlobalAlloc, Layout};

use core::cell::Cell;

#[cfg(feature = "alloc_counter_macro")]
pub use alloc_counter_macro::no_alloc;

// FIXME: be more no-std friendly
#[cfg(feature = "counters")]
thread_local!(static COUNTERS: Cell<(usize, usize, usize)> = Cell::new((0, 0, 0)));
#[cfg(feature = "no_alloc")]
thread_local!(static ALLOC_MODE: Cell<AllocMode> = Cell::new(AllocMode::Allow));

#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg(feature = "no_alloc")]
enum AllocMode {
    Allow,
    Deny,
    Forbid,
}

#[cfg(feature = "no_alloc")]
struct Guard(AllocMode);

#[cfg(feature = "no_alloc")]
impl Guard {
    #[inline(always)]
    fn new(newmode: AllocMode) -> Option<Self> {
        ALLOC_MODE.with(|mode| match mode.get() {
            AllocMode::Forbid => None,
            x => {
                mode.set(newmode);
                Some(Self(x))
            }
        })
    }
}

#[cfg(feature = "no_alloc")]
impl Drop for Guard {
    #[inline(always)]
    fn drop(&mut self) {
        ALLOC_MODE.with(|x| x.set(self.0));
    }
}

#[inline]
#[cfg(feature = "no_alloc")]
fn panicking() -> bool {
    #[cfg(feature = "std")]
    {
        std::thread::panicking()
    }

    #[cfg(not(feature = "std"))]
    false
}

pub struct AllocCounter<A>(pub A);

#[cfg(feature = "std")]
pub type AllocCounterSystem = AllocCounter<std::alloc::System>;
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
pub const AllocCounterSystem: AllocCounterSystem = AllocCounter(std::alloc::System);

unsafe impl<A> GlobalAlloc for AllocCounter<A>
where
    A: GlobalAlloc,
{
    #[inline(always)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[cfg(feature = "counters")]
        COUNTERS.with(|counters| {
            let mut was = counters.get();
            was.0 += 1;
            counters.set(was);
        });

        #[cfg(feature = "no_alloc")]
        ALLOC_MODE.with(|mode| {
            if mode.get() != AllocMode::Allow && !panicking() {
                panic!(
                    "Unexpected allocation of size {}, align {}.",
                    layout.size(),
                    layout.align(),
                );
            }
        });

        self.0.alloc(layout)
    }

    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        #[cfg(feature = "counters")]
        COUNTERS.with(|counters| {
            let mut was = counters.get();
            was.1 += 1;
            counters.set(was);
        });

        #[cfg(feature = "no_alloc")]
        ALLOC_MODE.with(|mode| {
            if mode.get() != AllocMode::Allow && !panicking() {
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

    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        #[cfg(feature = "counters")]
        COUNTERS.with(|counters| {
            let mut was = counters.get();
            was.2 += 1;
            counters.set(was);
        });

        // deallocate before we might panic
        self.0.dealloc(ptr, layout);

        #[cfg(feature = "no_alloc")]
        ALLOC_MODE.with(|mode| {
            if mode.get() != AllocMode::Allow && !panicking() {
                panic!(
                    "Unexpected deallocation of size {}, align {}.",
                    layout.size(),
                    layout.align(),
                );
            }
        });
    }
}

#[inline(always)]
#[cfg(feature = "counters")]
pub fn count_alloc<F, R>(f: F) -> ((usize, usize, usize), R)
where
    F: FnOnce() -> R,
{
    let (a, b, c) = COUNTERS.with(|counters| counters.get());
    let r = f();
    let (d, e, f) = COUNTERS.with(|counters| counters.get());

    ((d - a, e - b, f - c), r)
}

#[inline(always)]
#[cfg(feature = "no_alloc")]
pub fn allow_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = Guard::new(AllocMode::Allow);
    f()
}

#[inline(always)]
#[cfg(feature = "no_alloc")]
pub fn deny_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = Guard::new(AllocMode::Deny);
    f()
}

#[inline(always)]
#[cfg(feature = "no_alloc")]
pub fn forbid_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = Guard::new(AllocMode::Forbid);
    f()
}
