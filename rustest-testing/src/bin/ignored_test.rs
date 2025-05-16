use core::assert_eq;

use rustest::{test, *};

#[fixture]
fn Number() -> u32 {
    eprintln!("BUILD Number");
    5
}

#[test]
fn test_number(number: Number) {
    eprintln!("TEST test_number");
    assert_eq!(*number, 5);
}

#[test]
#[ignore]
fn test_ignored_number(number: Number) {
    eprintln!("TEST test_ignored_number");
    assert_eq!(*number, 5);
}

#[fixture(params:u32 = [5,6,42])]
fn ParamNumber(Param(n): Param) -> (u32, u32) {
    eprintln!("BUILD ParamNumber");
    (n, n * 2)
}

#[test]
fn test_param_number(number: ParamNumber) {
    let (input, expected) = *number;
    eprintln!("TEST test_param_number");
    assert_eq!(input * 2, expected);
}

#[test(ignore)]
fn test_ignored_param_number(number: ParamNumber) {
    let (input, expected) = *number;
    eprintln!("TEST test_ignored_param_number");
    assert_eq!(input * 2, expected);
}

#[main]
fn main() {}
