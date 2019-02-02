# alloc-counter

## Alloc counters

A redesign of the [Quick and Dirty Allocation Profiling
Tool](https://github.com/bspeice/qadapt).


### Features

* Count allocations, reallocations and deallocations individually with `count_alloc`.

* Allow, deny, and forbid use of the global allocator with `allow_alloc`, `deny_alloc` and
`forbid_alloc`.

* `#[no_alloc]` function attribute to deny and `#[no_alloc(forbid)]` to forbid use of the
global allocator.


### Limitations and known issues

* Methods must either take a reference to `self` or `Self` must be a `Copy` type.


### Usage

An `AllocCounter<A>` wraps an allocator `A` to individually count the number of calls to
`alloc`, `realloc`, and `dealloc`.

```rust
use alloc_counter::AllocCounter;

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
assert_eq!(
    count_alloc(|| {
        // no alloc
        let mut v = Vec::new();
        // alloc
        v.push(0);
        // realloc
        v.push(1);
        // dealloc
    })
    .0,
    (1, 1, 1)
);
```

To deny allocations for an expression use `deny_alloc`.

```rust
fn foo(b: Box<i32>) {
    deny_alloc(|| drop(b))
}
foo(Box::new(0));
```

Similar to Rust's lints if you deny something, you can still allow it. Unless you also forbid
it. However, forbidding and then allowing is valid to express, the forbid behavior will be
applied.

```rust
fn foo(b: Box<i32>) {
    forbid_alloc(|| allow_alloc(|| drop(b)))
}
foo(Box::new(0));
```

For added sugar you may use the `#[no_alloc]` attribute on functions, including methods with
self-binds. `#[no_alloc]` expands to calling `deny_alloc` and forcefully moves the parameters
into the checked block. `#[no_alloc(forbid)]` calls `forbid_alloc`.

```rust
#[no_alloc(forbid)]
fn foo(b: Box<i32>) {
    allow_alloc(|| drop(b))
}
foo(Box::new(0));
```

License: MIT OR Apache-2.0
