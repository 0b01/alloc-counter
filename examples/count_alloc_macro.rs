use alloc_counter::{count_alloc, AllocCounterSystem};

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

#[count_alloc]
fn test_vector(v: &mut Vec<usize>) {
    for i in 0..100 {
        v.push(i);
    }
}

fn log_allocs(cs: (usize, usize, usize)) {
    eprintln!("Save samples to disk or transmit over the network? {:?}", cs);
}

#[count_alloc(func = "log_allocs")]
fn main() {
    Box::new(0);
    Box::new([0usize; 120]);
    let mut v = Vec::new();
    test_vector(&mut v);
    test_vector(&mut v);
    test_vector(&mut v);
}
