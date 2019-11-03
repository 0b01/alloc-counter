# alloc_counter

## Alloc counters

A redesign of the [Quick and Dirty Allocation Profiling
Tool](https://github.com/bspeice/qadapt).

### Features

* Although `#[no_std]` is intended, it isn't currently supported due to the lack of portable
  `thread_local` implementation. This may be "fixed" with a feature flag to use a global atomic
  under the contract that the program is single threaded.

* Count allocations, reallocations and deallocations individually with `count_alloc`.

* Allow, deny, and forbid use of the global allocator with `allow_alloc`, `deny_alloc` and
`forbid_alloc`.

* `#[no_alloc]` function attribute to deny and `#[no_alloc(forbid)]` to forbid use of the
global allocator.

* `#[count_alloc]` function attribute to print the counts to stderr. Alternatively use
  `#[count_alloc(func = "my_function")]` where `my_function` accepts a triple of `usize`s
  and returns `()` to redirect the output.

### Limitations and known issues

* Methods must either take a reference to `self` or `Self` must be a `Copy` type.

* Ordinary and async functions must be treated differently. Use `count_alloc` for functions
  and `count_alloc_future` for futures.

### Usage

An `AllocCounter<A>` wraps an allocator `A` to individually count the number of calls to
`alloc`, `realloc`, and `dealloc`.

```rust
use alloc_counter::AllocCounter;

type MyAllocator = std::alloc::System;
const MyAllocator: MyAllocator = std::alloc::System;

#[global_allocator]
static A: AllocCounter<MyAllocator> = AllocCounter(MyAllocator);
```

Std-users may prefer to inherit their system's allocator.

```rust
use alloc_counter::AllocCounterSystem;

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;
```

To count the allocations of an expression, use `count_alloc`.

```rust
# use alloc_counter::{count_alloc, AllocCounterSystem};
# #[global_allocator]
# static A: AllocCounterSystem = AllocCounterSystem;
let (counts, v) = count_alloc(|| {
    // no alloc
    let mut v = Vec::new();
    // alloc
    v.push(0);
    // realloc
    v.push(1);
    // return the vector without deallocating
    v
});
assert_eq!(counts, (1, 1, 0));
```

To deny allocations for an expression use `deny_alloc`.

```rust,should_panic
# use alloc_counter::{deny_alloc, AllocCounterSystem};
# #[global_allocator]
# static A: AllocCounterSystem = AllocCounterSystem;
fn foo(b: Box<i32>) {
    // dropping causes a panic
    deny_alloc(|| drop(b))
}
foo(Box::new(0));
```

Similar to Rust's lints, you can still allow allocation inside a deny block.

```rust
# use alloc_counter::{allow_alloc, deny_alloc, AllocCounterSystem};
# #[global_allocator]
# static A: AllocCounterSystem = AllocCounterSystem;
fn foo(b: Box<i32>) {
    deny_alloc(|| allow_alloc(|| drop(b)))
}
foo(Box::new(0));
```

Forbidding allocations forces a panic even when `allow_alloc` is used.

```rust,should_panic
# use alloc_counter::{allow_alloc, forbid_alloc, AllocCounterSystem};
# #[global_allocator]
# static A: AllocCounterSystem = AllocCounterSystem;
fn foo(b: Box<i32>) {
    // panics because of outer `forbid`, even though drop happens in an allow block
    forbid_alloc(|| allow_alloc(|| drop(b)))
}
foo(Box::new(0));
```

For added sugar you may use the `#[no_alloc]` attribute on functions, including methods with
self-binds. `#[no_alloc]` expands to calling `deny_alloc` and forcefully moves the parameters
into the checked block. `#[no_alloc(forbid)]` calls `forbid_alloc`.

```rust,should_panic
# #[cfg(not(feature = "macros"))]
# panic!("macros feature disabled");
# #[cfg(feature = "macros")] {
# use alloc_counter::{allow_alloc, no_alloc, AllocCounterSystem};
# #[global_allocator]
# static A: AllocCounterSystem = AllocCounterSystem;
#[no_alloc(forbid)]
fn foo(b: Box<i32>) {
    allow_alloc(|| drop(b))
}
foo(Box::new(0));
# }
```

License: MIT OR Apache-2.0
