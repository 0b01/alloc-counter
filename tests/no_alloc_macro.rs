#![cfg(feature = "macros")]

use alloc_counter::*;

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

#[test]
#[cfg_attr(debug_assertions, should_panic)]
fn no_alloc_try_to_alloc() {
    #[no_alloc]
    fn foo() {
        Box::new(0);
    }
    foo();
}

#[test]
#[should_panic]
fn no_alloc_dealloc_after_move() {
    #[no_alloc]
    fn foo(_b: Box<i32>) {}
    foo(Box::new(0));
}

#[test]
fn no_alloc_then_allow() {
    #[no_alloc]
    fn foo(b: Box<i32>) {
        allow_alloc(|| drop(b))
    }

    foo(Box::new(0));
}

#[test]
#[should_panic]
fn no_alloc_forbid_then_allow() {
    #[no_alloc(forbid)]
    fn foo(b: Box<i32>) {
        allow_alloc(|| drop(b))
    }
    foo(Box::new(0));
}

#[derive(Clone, Copy)]
struct Foo;

impl Foo {
    #[no_alloc]
    fn foo(self) {}

    #[no_alloc]
    fn bar(&self) {}

    #[no_alloc]
    fn baz(self, mut a: usize) -> usize {
        a += 1;
        a
    }
}
