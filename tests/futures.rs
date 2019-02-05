#![cfg(feature = "alloc_counter_macro")]
#![feature(futures_api, async_await, await_macro, generators, generator_trait)]

use alloc_counter::*;
//use futures::executor::block_on;

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

#[test]
#[ignore]
fn async_fn() {
    #[no_alloc]
    async fn foo() -> i32 {
        0
    }

    async fn bar() {
        await!(foo());
    }

    // FIXME: block_on(foo())
}

#[test]
#[should_panic]
#[ignore]
fn async_fn_bad() {
    #[no_alloc]
    async fn foo() -> i32 {
        *Box::new(0)
    }

    async fn bar() {
        await!(foo());
    }

    // FIXME: block_on(foo())
}
