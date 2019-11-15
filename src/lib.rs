#![cfg_attr(not(feature = "std"), no_std)]
#![feature(doc_cfg)]
#![cfg_attr(rustdoc, feature(external_doc))]
#![cfg_attr(rustdoc, doc(include = "../README.md"))]
#![cfg_attr(not(rustdoc), doc = "external documentation in README.md")]
#![warn(missing_docs)]

extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};

macro_rules! counters {
    ($a:tt,$r:tt,$d:tt) => {
        Counters {alloc_count:$a, realloc_count: $r, dealloc_count: $d, ..}
    };
    ($a:tt,$r:tt,$d:tt,$as:tt,$rs:tt,$ds:tt) => {
        Counters {
            alloc_count:$a, realloc_count: $r, dealloc_count: $d,
            alloc_size:$as, realloc_size: $rs, dealloc_size: $ds,
        }
    };
}

use core::{
    cell::Cell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "macros")]
pub use alloc_counter_macro::{count_alloc, no_alloc};

// FIXME: static atomics for single-threaded no_std?
thread_local!(static COUNTERS: Cell<Counters> = Cell::default());
thread_local!(static MODE: Cell<AllocMode> = Cell::new(AllocMode::Count));

/// A tuple of the counts; respectively allocations, reallocations, and deallocations.
#[derive(Clone, Copy, Default)]
pub struct Counters {
    /// count of allocations
    pub alloc_count: usize,
    /// allocated size
    pub alloc_size: usize,
    /// count of reallocations
    pub realloc_count: usize,
    /// reallocated size
    pub realloc_size: usize,
    /// count of deallocations
    pub dealloc_count: usize,
    /// deallocated size
    pub dealloc_size: usize,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
/// Configure how allocations are counted
pub enum AllocMode {
    /// Do not count allocations (unless a higher scope is forbidding them)
    Ignore,
    /// Do count allocations
    Count,
    /// Count all allocations even if the contained code attempts to allow them.
    CountAll,
}

/// An allocator that tracks allocations, reallocations, and deallocations in live code.
/// It uses another backing allocator for actual heap management.
pub struct AllocCounter<A>(pub A);

#[cfg(feature = "std")]
/// Type alias for an `AllocCounter` backed by the operating system's default allocator
pub type AllocCounterSystem = AllocCounter<std::alloc::System>;
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
/// An allocator that counts allocations, reallocations, and deallocations in live code.
/// It uses the operating system as a backing implementation for actual heap management.
pub const AllocCounterSystem: AllocCounterSystem = AllocCounter(std::alloc::System);

unsafe impl<A> GlobalAlloc for AllocCounter<A>
where
    A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if MODE.with(Cell::get) != AllocMode::Ignore {
            COUNTERS.with(|x| {
                let mut c = x.get();
                c.alloc_count += 1;
                c.alloc_size += layout.size();
                x.set(c);
            });
        }

        self.0.alloc(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if MODE.with(Cell::get) != AllocMode::Ignore {
            COUNTERS.with(|x| {
                let mut c = x.get();
                c.realloc_count += 1;
                c.realloc_size = c.realloc_size + new_size - layout.size(); // TODO: not sure if correct
                x.set(c);
            });
        }

        self.0.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if MODE.with(Cell::get) != AllocMode::Ignore {
            COUNTERS.with(|x| {
                let mut c = x.get();
                c.dealloc_count += 1;
                c.dealloc_size += layout.size();
                x.set(c);
            });
        }

        self.0.dealloc(ptr, layout);
    }
}

struct Guard(AllocMode);

impl Drop for Guard {
    fn drop(&mut self) {
        MODE.with(|x| x.set(self.0))
    }
}

impl Guard {
    fn new(mode: AllocMode) -> Self {
        Guard(MODE.with(|x| {
            if x.get() != AllocMode::CountAll {
                x.replace(mode)
            } else {
                AllocMode::CountAll
            }
        }))
    }
}

/// Count the allocations, reallocations, and deallocations that happen during execution of a
/// closure.
///
/// Example:
///
/// ```rust
/// # use alloc_counter::{AllocCounterSystem, count_alloc};
/// # #[global_allocator]
/// # static A: AllocCounterSystem = AllocCounterSystem;
/// let (counts, result) = count_alloc(|| {
///     // no alloc
///     let mut v = Vec::new();
///     // alloc
///     v.push(0);
///     // realloc
///     v.push(8);
///     // return 8 from the closure
///     v.pop().unwrap()
///     // dealloc on dropping v
/// });
/// assert_eq!(result, 8);
/// assert_eq!(counts, (1, 1, 1));
/// ```
pub fn count_alloc<F, R>(f: F) -> (Counters, R)
where
    F: FnOnce() -> R,
{
    let counters!(a1, r1, d1, as1, rs1, ds1) = COUNTERS.with(Cell::get);
    let r = f();
    let counters!(a2, r2, d2, as2, rs2, ds2) = COUNTERS.with(Cell::get);

    (
        Counters {
            alloc_count: a2 - a1,
            realloc_count: r2 - r1,
            dealloc_count: d2 - d1,
            alloc_size: as2 - as1,
            realloc_size: rs2 - rs1,
            dealloc_size: ds2 - ds1,
        },
        r
    )
}

/// Count the allocations, reallocations, and deallocations that happen duringexecution of a
/// future.
///
/// Example:
///
/// ```rust
/// # use alloc_counter::{AllocCounterSystem, count_alloc_future};
/// # use futures_executor::block_on;
/// # #[global_allocator]
/// # static A: AllocCounterSystem = AllocCounterSystem;
/// let (counts, result) = block_on(count_alloc_future(async { Box::new(0); }));
/// assert_eq!(counts, (1, 0, 1));
/// ```
pub fn count_alloc_future<F>(future: F) -> AsyncGuard<F> {
    AsyncGuard {
        future,
        counts: Default::default(),
    }
}

/// A future-wrapper which counts the allocations, reallocations, and deallocations that occur
/// while the future is evaluating.
pub struct AsyncGuard<F> {
    counts: Counters,
    future: F,
}

impl<F> AsyncGuard<F>
where
    F: Future,
{
    pin_utils::unsafe_pinned!(future: F);

    pin_utils::unsafe_pinned!(counts: Counters);
}

impl<F> Future for AsyncGuard<F>
where
    F: Future,
{
    type Output = (Counters, F::Output);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let (counters!(a, r, d), x) = count_alloc(|| self.as_mut().future().poll(cx));
        let counts = self.counts().get_mut();
        counts.alloc_count += a;
        counts.realloc_count += r;
        counts.dealloc_count += d;
        match x {
            Poll::Ready(x) => Poll::Ready((*counts, x)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Apply the allocation mode against a function/closure. Panicking if any allocations,
/// reallocations or deallocations occur. (Use `guard_future` for futures)
pub fn guard_fn<F, R>(mode: AllocMode, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = Guard::new(mode);
    let (counters!(a, r, d,r#as,rs,ds), x) = count_alloc(f);
    if mode != AllocMode::Ignore && (a, r, d) != (0, 0, 0) {
        panic!(
            "allocations: [{},{}], reallocations: [{},{}], deallocations: [{},{}]",
            a,r#as, r,rs, d,ds
        )
    }
    x
}

/// Apply the allocation mode against a future. Panicking if any allocations,
/// reallocations or deallocations occur. (Use `guard_fn` for functions)
pub async fn guard_future<F>(mode: AllocMode, f: F) -> F::Output
where
    F: Future,
{
    let _guard = Guard::new(mode);
    let (counters!{a, r, d}, x) = count_alloc_future(f).await;
    if mode != AllocMode::Ignore && (a, r, d) != (0, 0, 0) {
        panic!(
            "alloc_count: {}, realloc_count: {}, dealloc_count: {}",
            a, r, d,
        )
    }
    x
}

/// Allow allocations for a closure, even if running in a deny closure.
/// Allocations during a forbid closure will still cause a panic.
///
/// Examples:
///
/// ```rust
/// # use alloc_counter::{AllocCounterSystem, allow_alloc, deny_alloc};
/// # #[global_allocator]
/// # static A: AllocCounterSystem = AllocCounterSystem;
/// fn foo(b: Box<i32>) {
///     // safe since the drop happens in an `allow` closure
///     deny_alloc(|| allow_alloc(|| drop(b)))
/// }
/// foo(Box::new(0));
/// ```
///
/// ```rust,should_panic
/// # use alloc_counter::{AllocCounterSystem, forbid_alloc, allow_alloc};
/// # #[global_allocator]
/// # static A: AllocCounterSystem = AllocCounterSystem;
/// fn foo(b: Box<i32>) {
///     // panics because of outer `forbid`, even though drop happens in an allow block
///     forbid_alloc(|| allow_alloc(|| drop(b)))
/// }
/// foo(Box::new(0));
/// ```
pub fn allow_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    guard_fn(AllocMode::Ignore, f)
}

/// Panic on any allocations during the provided closure. If code within the closure
/// calls `allow_alloc`, allocations are allowed within that scope.
///
/// Examples:
///
/// ```rust,should_panic
/// # use alloc_counter::{AllocCounterSystem, deny_alloc};
/// # #[global_allocator]
/// # static A: AllocCounterSystem = AllocCounterSystem;
/// // panics due to `Box` forcing a heap allocation
/// deny_alloc(|| Box::new(0));
/// ```
///
/// ```rust
/// # use alloc_counter::{AllocCounterSystem, allow_alloc, deny_alloc};
/// # #[global_allocator]
/// # static A: AllocCounterSystem = AllocCounterSystem;
/// fn foo(b: Box<i32>) {
///     // safe since the drop happens in an `allow` closure
///     deny_alloc(|| allow_alloc(|| drop(b)));
/// }
/// foo(Box::new(0));
/// ```
pub fn deny_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    guard_fn(AllocMode::Count, f)
}

/// Panic on any allocations during the provided closure, even if the closure contains
/// code in an `allow_alloc` guard.
///
/// Example:
///
/// ```rust,should_panic
/// # use alloc_counter::{AllocCounterSystem, forbid_alloc, allow_alloc};
/// # #[global_allocator]
/// # static A: AllocCounterSystem = AllocCounterSystem;
/// fn foo(b: Box<i32>) {
///     // panics because of outer `forbid` even though drop happens in an allow closure
///     forbid_alloc(|| allow_alloc(|| drop(b)))
/// }
/// foo(Box::new(0));
pub fn forbid_alloc<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    guard_fn(AllocMode::CountAll, f)
}
