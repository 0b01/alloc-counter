use alloc_counter::*;

#[global_allocator]
static A: AllocCounterSystem = ALLOC_COUNTER_SYSTEM;

#[test]
#[should_panic]
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
