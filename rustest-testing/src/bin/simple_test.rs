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

#[test]
fn test_param_number_bis(number: ParamNumber) {
    let (input, expected) = *number;
    eprintln!("TEST test_param_number_bis");
    assert_eq!(input * 2, expected);
}

#[fixture(params:u32 = [5,6,42], scope=global)]
fn ParamGlobalNumber(Param(n): Param) -> (u32, u32) {
    eprintln!("BUILD ParamGlobalNumber");
    (n, n * 2)
}

#[test]
fn test_param_global_number(number: ParamGlobalNumber) {
    let (input, expected) = *number;
    eprintln!("TEST test_param_global_number");
    assert_eq!(input * 2, expected);
}

#[test]
fn test_param_global_number_bis(number: ParamGlobalNumber) {
    let (input, expected) = *number;
    eprintln!("TEST test_param_global_number_bis");
    assert_eq!(input * 2, expected);
}

#[main]
fn main() {}
