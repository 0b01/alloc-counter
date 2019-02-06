use alloc_counter::{count_alloc, AllocCounterSystem};

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

#[count_alloc]
fn test_vector(v: &mut Vec<usize>) {
    for i in 0..100 {
        v.push(i);
    }
}

#[count_alloc]
fn main() {
    Box::new(0);
    Box::new([0usize; 120]);
    let mut v = Vec::new();
    test_vector(&mut v);
    test_vector(&mut v);
    test_vector(&mut v);
}
