#![cfg(feature = "macros")]

use alloc_counter::{no_alloc, AllocCounterSystem};
use futures_executor::block_on;

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

#[test]
fn async_fn() {
    #[no_alloc]
    async fn foo() -> i32 {
        0
    }

    async fn bar() {
        foo().await;
    }

    block_on(bar());
}

#[test]
#[cfg_attr(debug_assertions, should_panic)]
fn async_fn_bad() {
    #[no_alloc]
    async fn foo() -> i32 {
        *Box::new(0)
    }

    async fn bar() {
        foo().await;
    }

    block_on(bar());
}
