use rustest::{test, *};
use rustest_fixtures::Global;

use std::sync::atomic::{AtomicU32, Ordering};
static INC_NUMBER: AtomicU32 = AtomicU32::new(0);

#[fixture]
fn IncNumber() -> u32 {
    println!("Build number");
    INC_NUMBER.fetch_add(1, Ordering::Relaxed)
}

#[test]
fn test_global_0(number: Global<IncNumber>) {
    assert_eq!(*number, 0);
}

#[test]
fn test_global_1(number: Global<IncNumber>) {
    assert_eq!(*number, 0);
}

#[main]
fn main() {}
