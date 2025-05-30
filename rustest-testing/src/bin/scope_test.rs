use rustest::{test, *};
use rustest_fixtures::*;

use std::sync::atomic::{AtomicU32, Ordering};

static INC_NUMBER: AtomicU32 = AtomicU32::new(0);

fn get_new_number() -> u32 {
    INC_NUMBER.fetch_add(1, Ordering::Relaxed)
}

#[fixture]
fn ScopeNumber() -> u32 {
    let number = get_new_number();
    eprintln!("BUILD scope number:{number}");
    number
}

#[fixture(scope=matrix)]
fn MatrixNumber() -> u32 {
    let number = get_new_number();
    eprintln!("BUILD matrix number:{number}");
    number
}

#[fixture(scope=test)]
fn TestNumber() -> u32 {
    let number = get_new_number();
    eprintln!("BUILD test number:{number}");
    number
}

#[fixture(scope=global)]
fn GlobalNumber() -> u32 {
    let number = get_new_number();
    eprintln!("BUILD global number:{number}");
    number
}

#[fixture(params:u32=[5,6])]
fn ParamNumber(Param(n): Param) -> u32 {
    eprintln!("BUILD param number:{n}");
    n
}

#[fixture]
fn IntermediateFixture<N>(scope_number: N, _p: ParamNumber) -> u32
where
    N: SubFixture<Type = u32>,
{
    *scope_number
}

// Scope number must be unique, even when same test using it in different test case.
#[test]
fn test_scope_number(scope_number: IntermediateFixture<ScopeNumber>, _other_number: ParamNumber) {
    eprintln!("TEST scope number:{}", *scope_number);
}

// MatrixUnique number are unique for a matrix.
// MatrixNumber is used in 3 matrix:
// - In IntermediateFixture, used in test_matrix_number_1.
// - Directly in test_matrix_number_1.
// - Directly in test_matrix_number_2.
#[test]
fn test_matrix_number_1(
    intermediate_matrix_number: IntermediateFixture<MatrixNumber>,
    matrix_number: MatrixNumber,
) {
    eprintln!("TEST matrix number:{}", *matrix_number);
    eprintln!("TEST matrix number:{}", *intermediate_matrix_number);
}
#[test]
fn test_matrix_number_2(matrix_number: MatrixNumber, _other_number: ParamNumber) {
    eprintln!("TEST matrix number:{}", *matrix_number);
}

// Test scope are unique per test, so only two (different) because we have two tests.
// So me build only two number.
// When testing:
// - first test get 4 equals numbers (2 per test case, 2 test cases)
// - second test get 2 equals numbers (2 test cases)
#[test]
fn test_test_number_1(
    intermediate_test_number: IntermediateFixture<TestNumber>,
    test_number: TestNumber,
) {
    eprintln!("TEST test number:{}", *test_number);
    eprintln!("TEST test number:{}", *intermediate_test_number);
}

#[test]
fn test_test_number_2(test_number: TestNumber, _other_number: ParamNumber) {
    eprintln!("TEST test number:{}", *test_number);
}

// Global scope are unique for the global scope, so only one per definition.
#[test]
fn test_global_number_1(
    intermediate_global_number: IntermediateFixture<GlobalNumber>,
    global_number: GlobalNumber,
) {
    eprintln!("TEST global number:{}", *global_number);
    eprintln!("TEST global number:{}", *intermediate_global_number);
}

#[test]
fn test_global_number_2(global_number: GlobalNumber, _other_number: ParamNumber) {
    eprintln!("TEST global number:{}", *global_number);
}

// Global scope are unique for the global scope, so only one per definition.
#[test]
fn test_make_global_number_1(
    intermediate_global_number: IntermediateFixture<Global<ScopeNumber>>,
    global_number: Global<ScopeNumber>,
) {
    eprintln!("TEST make global number:{}", *global_number);
    eprintln!("TEST make global number:{}", *intermediate_global_number);
}

#[test]
fn test_make_global_number_2(global_number: Global<ScopeNumber>, _other_number: ParamNumber) {
    eprintln!("TEST make global number:{}", *global_number);
}

// Here, what is global is `IntermediateFixture<ScopeNumber>`. But as the IntermediateFixture is
// parametrized with another param value, the ScopeNumber is build twice, even if the IntermediateFixture is cached.
#[test]
fn test_make_global_number_wrong_1(
    intermediate_global_number: Global<IntermediateFixture<ScopeNumber>>,
    global_number: Global<ScopeNumber>,
) {
    eprintln!("TEST make global wrong number:{}", *global_number);
    eprintln!(
        "TEST make global wrong number:{}",
        *intermediate_global_number
    );
}

// But the intermediatFixture is not rebuild here.
#[test]
fn test_make_global_number_wrong_2(
    intermediate_global_number: Global<IntermediateFixture<ScopeNumber>>,
) {
    eprintln!(
        "TEST make global wrong number:{}",
        *intermediate_global_number
    );
}
#[main]
fn main() {}
