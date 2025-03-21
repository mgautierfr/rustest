use core::{assert_eq, sync::atomic::AtomicU32};
use std::process::Stdio;

use rustest::{Result, fixture, main, test};

// Tests are simply marked with #[test], as any classic rust integration tests
#[test]
fn simple_test() {
    assert!(
        5 * 6 == 30,
        "Rust should be able to do a simple multiplication"
    )
}

#[test]
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

#[test]
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
#[fixture(global)]
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
#[fixture(global)]
fn IncNumber3(source: IncNumber) -> u32 {
    *source
}

#[test]
fn test_fixture_inc_number3_0(number: IncNumber3) {
    assert_eq!(*number, 3)
}
#[test]
fn test_fixture_inc_number3_1(number: IncNumber3) {
    assert_eq!(*number, 3)
}
#[test]
fn test_fixture_inc_number3_2(number: IncNumber3) {
    assert_eq!(*number, 3)
}

#[fixture]
fn RunningProcess() -> std::io::Result<Box<std::process::Child>> {
    Ok(Box::new(
        std::process::Command::new("bash")
            .stdout(Stdio::piped())
            .arg("-c")
            .arg("while true; do sleep 1; done")
            .spawn()?,
    ))
}

// Teardown can be implemented using the Drop trait.
// Either on the fixture itself (as it is now)
// or on the returned type of the setup method.
impl Drop for RunningProcess {
    fn drop(&mut self) {
        self.0.kill().unwrap()
    }
}

#[test]
fn test_with_process(a_process: RunningProcess) -> Result {
    println!("Process id: {}", a_process.id());

    Ok(())
}

// Global fixtures store their values in an `Arc`, so if you want to impl Drop you have
// to impl it on thi inner value. Here, we need a new type to be able to impl it "on" Child.

#[derive(Debug)]
struct DropChild(std::process::Child);

impl From<std::process::Child> for DropChild {
    fn from(v: std::process::Child) -> Self {
        Self(v)
    }
}

impl std::ops::Deref for DropChild {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// We have to impl Drop on DropChild, not on GlobalRunningProcess.
impl Drop for DropChild {
    fn drop(&mut self) {
        self.0.kill().unwrap()
    }
}

#[fixture(global)]
fn GlobalRunningProcess() -> std::io::Result<DropChild> {
    std::process::Command::new("bash")
        .stdout(Stdio::piped())
        .arg("-c")
        .arg("while true; do sleep 1; done")
        .spawn()
        .map(|c| c.into())
}

#[test]
fn test_with_global_process(a_process: GlobalRunningProcess) -> Result {
    println!("Process id: {}", a_process.id());

    Ok(())
}

main! {}
