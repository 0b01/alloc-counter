use alloc_counter::*;

#[global_allocator]
static A: AllocCounterSystem = ALLOC_COUNTER_SYSTEM;

#[test]
fn count_0() {
    assert_eq!(count_alloc(|| 0).0, (0, 0, 0));
}

#[test]
fn count_1() {
    assert_eq!(count_alloc(|| Box::new(0)).0, (1, 0, 0));
}

#[test]
fn count_2() {
    assert_eq!(
        count_alloc(|| {
            // alloc
            Box::new(0);
            // dealloc
        })
        .0,
        (1, 0, 1)
    );
}

#[test]
fn count_3() {
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
}

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
