use core::{assert_eq, sync::atomic::AtomicU32};
use std::process::Stdio;

use rustest::{Result, SubFixture, fixture, main, test};

// Tests are simply marked with #[test], as any classic rust integration tests
#[test]
fn simple_test() {
    assert!(
        5 * 6 == 30,
        "Rust should be able to do a simple multiplication"
    )
}

// Test expected to fail can be marked with #[xfail] attribute
#[test]
#[xfail]
fn simple_test_failing() {
    assert!(
        5 * 4 == 30,
        "Rust should be able to do a simple multiplication"
    )
}

#[test]
fn return_test() -> Result {
    Ok(())
}

// Test expected to fail can also be marked with test(xfail) attribute
#[test(xfail)]
fn return_test_failing() -> Result {
    Err(std::io::Error::other("Dummy error"))?;
    Ok(())
}

// Fixture can simply defined as a function returning a value
#[fixture]
fn ANumber() -> u32 {
    5
}

// Fixtures are referenced by they name as input type.
// Fixtures deref to their inner value (u32 here).
#[test]
fn test_fixture_number(number: ANumber) {
    assert_eq!(*number, 5)
}

// Fixture's name can be specified with the `name` attribute.
// The function's name is useless in this case and can be anything.
#[fixture(name = ANewNumber)]
fn setup() -> u32 {
    6
}

// Fixtures are referenced by they name as input type.
// Fixtures deref to their inner value (u32 here).
#[test]
fn test_fixture_new_number(number: ANewNumber) {
    assert_eq!(*number, 6)
}

static INC_NUMBER: AtomicU32 = AtomicU32::new(0);

// Fixture are setup each time we need it.
#[fixture]
fn IncNumber() -> u32 {
    INC_NUMBER.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}

// As we use several time the fixture, we have a new number everytime.
#[test]
fn test_fixture_inc_number_0(number: IncNumber) {
    assert_eq!(*number, 0)
}
#[test]
fn test_fixture_inc_number_1(number: IncNumber) {
    assert_eq!(*number, 1)
}
#[test]
fn test_fixture_inc_number_2(number: IncNumber) {
    assert_eq!(*number, 2)
}

static INC_NUMBER2: AtomicU32 = AtomicU32::new(0);

// Global fixture are setup only once.
#[fixture(scope=global)]
fn IncNumber2() -> u32 {
    INC_NUMBER2.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}

// As the fixture is setup once, we always have the same number.
#[test]
fn test_fixture_inc_number2_0(number: IncNumber2) {
    assert_eq!(*number, 0)
}
#[test]
fn test_fixture_inc_number2_1(number: IncNumber2) {
    assert_eq!(*number, 0)
}
#[test]
fn test_fixture_inc_number2_2(number: IncNumber2) {
    assert_eq!(*number, 0)
}

// Fixtures can use other fixtures as source.
#[fixture(scope=global)]
fn IncNumber3(source: IncNumber) -> u32 {
    *source
}

#[test]
fn test_fixture_inc_number3_0(number3: IncNumber3) {
    assert_eq!(*number3, 3)
}
#[test]
fn test_fixture_inc_number3_1(number3: IncNumber3) {
    assert_eq!(*number3, 3)
}
#[test]
fn test_fixture_inc_number3_2(number3: IncNumber3) {
    assert_eq!(*number3, 3)
}

#[derive(Debug)]
struct ProcessChild(pub std::process::Child);

// This fixture is a sub process stucks in a infinite loop.
// If we don't kill it when we don't need it, it will be keeped ulive and zombyfied at end of tests.
// Teardown must be a function taking a `&mut value` and droping it as it has to.
#[fixture(teardown=|v| v.0.kill().unwrap())]
fn RunningProcess() -> std::io::Result<Box<ProcessChild>> {
    Ok(Box::new(ProcessChild(
        std::process::Command::new("bash")
            .stdout(Stdio::piped())
            .arg("-c")
            .arg("while true; do sleep 1; done")
            .spawn()?,
    )))
}

#[test]
fn test_with_process(a_process: RunningProcess) -> Result {
    println!("Process id: {}", a_process.0.id());

    Ok(())
}

// By default, a new fixtures is created each time we request it.
// So Double get a new number and returns is double
#[fixture]
fn Double(source: IncNumber) -> u32 {
    *source * 2
}

// So Double is the double of a new IncNumber, so (previous incNumber + 1)*2
#[test]
fn test_double_unique(a_number: IncNumber, its_double: Double) {
    assert_eq!((*a_number + 1) * 2, *its_double);
}

// We can for a fixture to be instanciated only once per test
#[fixture(scope=test)]
fn IncNumberLocal(source: IncNumber) -> u32 {
    *source
}
#[fixture]
fn DoubleLocal(source: IncNumberLocal) -> u32 {
    *source * 2
}

#[test]
fn test_double_local(a_number: IncNumberLocal, its_double: DoubleLocal) {
    assert_eq!(*a_number * 2, *its_double);
}

// Fixture can be generic by other fixtures.
// This fixture need a fixture source of type u32 and return its double,
// but the exact type of the source is not known yet.
#[fixture]
fn DoubleGeneric<Source>(source: Source) -> u32
where
    Source: SubFixture<Type = u32>,
{
    *source * 2
}

// This is equivalent to test `test_double_unique` but using generic
#[test]
fn test_double_unique_gen(a_number: IncNumber, its_double: DoubleGeneric<IncNumber>) {
    assert_eq!((*a_number + 1) * 2, *its_double);
}

// This is equivalent to test `test_double_local` but using generic
#[test]
fn test_double_local_gen(a_number: IncNumberLocal, its_double: DoubleGeneric<IncNumberLocal>) {
    assert_eq!(*a_number * 2, *its_double);
}

type MyDoubleFixture = DoubleGeneric<IncNumberLocal>;
#[test]
fn test_double_typedef(a_number: IncNumberLocal, its_double: MyDoubleFixture) {
    assert_eq!(*a_number * 2, *its_double);
}

// Fixture can be parametrized.
// The Fixture argument must be named `param` and have the type of `Param`.
#[fixture(scope=test, params:u32=[1,5])]
fn Parametrized(param: Param) -> u32 {
    *param
}

// This will create two tests:
// - test_param[Parametrized:1]
// - test_param[Parametrized:5]
#[test]
fn test_param(a_number: Parametrized) {
    assert!([1, 5].contains(&a_number));
}

// This will create two tests:
// - test_param[DoubleGeneric:2]
// - test_param[DoubleGeneric:10]
#[test]
fn test_param_double(a_number: DoubleGeneric<Parametrized>) {
    assert!([2, 10].contains(&a_number));
}

// Tests can be parametrized too.
// - test_param2[Parametrized:1]
// - test_param2[Parametrized:5]
// The test argument must be named `param` and have the type of `Param`.

#[test(params:u32=[1,5])]
fn test_param2(p: Param) {
    assert!([1, 5].contains(&p));
}

#[test(params:(u32, u32)=[
     (0, 0),
     (1, 1),
     (2, 1),
     (3, 2),
     (4, 3),
     (5, 5),
     (6, 8),
 ])]
fn fibonacci_test(Param((input, expected)): Param) {
    assert_eq!(expected, fibonacci(input))
}

fn fibonacci(input: u32) -> u32 {
    match input {
        0 => 0,
        1 => 1,
        n => fibonacci(n - 2) + fibonacci(n - 1),
    }
}

#[main]
fn main() {}
