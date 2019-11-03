use alloc_counter::*;

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

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
    let counts = count_alloc(|| {
        // alloc
        Box::new(0);
        // dealloc
    })
    .0;

    if cfg!(debug_assertions) {
        assert_eq!(counts, (1, 0, 1));
    } else {
        assert_eq!(counts, (0, 0, 0));
    }
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
