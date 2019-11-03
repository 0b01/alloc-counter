use alloc_counter::*;

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

#[test]
fn allow() {
    Box::new(0);
}

#[test]
#[should_panic]
fn deny() {
    deny_alloc(|| Box::new(0));
}

#[test]
#[should_panic]
fn forbid() {
    forbid_alloc(|| Box::new(0));
}

#[test]
fn deny_then_allow() {
    deny_alloc(|| allow_alloc(|| Box::new(0)));
}

#[test]
fn forbid_then_allow() {
    forbid_alloc(|| allow_alloc(|| 0));
}

#[test]
#[should_panic]
fn forbid_sticks() {
    forbid_alloc(|| allow_alloc(|| Box::new(0)));
}
