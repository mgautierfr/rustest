#![allow(clippy::test_attr_in_doctest)]

mod fixture;
mod test;
use fixture::FixtureRegistry;
#[doc(hidden)]
pub use fixture::SharedFixtureValue;
pub use fixture::{Fixture, FixtureCreationError, FixtureDisplay, FixtureMatrix, FixtureScope};
#[doc(hidden)]
pub use test::{InnerTestResult, IntoError};
pub use test::{Result, Test, TestContext};

/// Function creating a set of [Test] from a [TestContext].
///
/// Classic function will resolve fixture matrix and generate a [Test] per combination.
/// Such functions are implement by [test]
pub type TestCtorFn =
    fn(&mut TestContext) -> ::std::result::Result<Vec<Test>, FixtureCreationError>;

/// Build tests from `test_ctors` and run them.
///
/// You should not directly call it directly.
/// Use [main] attribute on an empty main function.
pub fn run_tests(test_ctors: &[TestCtorFn]) -> std::process::ExitCode {
    use libtest_mimic::{Arguments, run};
    let args = Arguments::from_args();

    let mut global_registry = FixtureRegistry::new();

    let tests: ::std::result::Result<Vec<_>, FixtureCreationError> = test_ctors
        .iter()
        .map(|test_ctor| {
            let mut test_registry = FixtureRegistry::new();
            let mut ctx = TestContext::new(&mut global_registry, &mut test_registry);
            test_ctor(&mut ctx)
        })
        .collect();

    let tests = match tests {
        Ok(tests) => tests.into_iter().flatten().map(|t| t.into()).collect(),
        Err(e) => {
            eprintln!("Failed to create fixture {}: {}", e.fixture_name, e.error);
            return std::process::ExitCode::FAILURE;
        }
    };
    let conclusion = run(&args, tests);
    conclusion.exit_code()
}

/// Define a fixture that you can use in all `rustest`'s test arguments. You should just mark your
/// function as `#[fixture]` and then use it as a test's argument. Fixture functions can also
/// use other fixtures.
///
/// Let's see a trivial example:
///
/// ```
/// use rustest::{test ,*};
///
/// #[fixture]
/// fn TwentyOne() -> i32 { 21 }
///
/// #[fixture]
/// fn Two() -> i32 { 2 }
///
/// #[fixture]
/// fn Injected(twenty_one: TwentyOne, two: Two) -> i32 { *twenty_one * *two }
///
/// #[test]
/// fn the_test(injected: Injected) {
///     assert_eq!(42, *injected)
/// }
///
/// #[main]
/// fn main() {}
/// ```
///
/// # Global Fixture
///
/// Especially in integration tests there are cases where you need a fixture that is called just once
/// for every tests. `rustest` provides you a way to put the fixture in the global scope.
///
/// If you mark your fixture with this attribute, then `rustest` will compute a the fixture only once
/// and use it in all your tests that need this fixture.
///
/// In follow example all tests share the same reference to the `42` static value.
///
/// ```
/// use rustest::{test,*};
///
/// #[fixture(scope=global)]
/// fn OnceFixture() -> i32 { 42 }
///
/// // Take care!!! You need to use a reference to the fixture value
///
/// #[test]
/// fn one_test(once_fixture: OnceFixture) {
///     assert_eq!(42, *once_fixture)
/// }
///
/// #[test]
/// fn other_test(once_fixture: OnceFixture) {
///     assert_eq!(42, *once_fixture)
/// }
///
/// #[main]
/// fn main() {}
/// ```
///
/// ## Rename
/// ```
/// # use rustest::{test, *};
/// #[fixture(name=MyFixture)]
/// fn long_and_boring_descriptive_name() -> i32 { 42 }
///
/// #[test]
/// fn the_test(value: MyFixture) {
///     assert_eq!(42, *value)
/// }
///
/// #[main]
/// fn main() {}
/// ```
///
/// ## Partial Injection
///
/// ```
/// use rustest::{test ,*};
/// #[fixture]
/// fn Base1() -> i32 { 1 }
///
/// #[fixture]
/// fn Base2() -> i32 { 2 }
///
/// #[fixture]
/// fn Double<S: Fixture<Type=i32>> (base: S) -> i32
/// { 2 * *base }
///
/// #[test]
/// fn test_double1(value: Double<Base1>) { assert_eq!(2, *value) }
///
/// #[test]
/// fn test_double2(value: Double<Base2>) { assert_eq!(4, *value) }
///
/// #[main]
/// fn main() {}
/// ```
pub use rustest_macro::fixture;

/// The attribute that you should use for your tests.
///
/// You must explicitly use `rustest::test` (or mark the function with `#[rustest::test]`).
/// If not, rust will detect a ambigous name for `test` and refuse to compile.
///
/// ```compile_fail
/// use rustest::*;
///
/// # #[fixture]
/// # fn Injected() -> i32 { 42 }
/// #[test]
/// fn the_test(injected: Injected) {
///     assert_eq!(42, *injected)
/// }
/// #[main]
/// fn main() {}
/// ```
/// ```
/// use rustest::{test,*};
///
/// # #[fixture]
/// # fn Injected() -> i32 { 42 }
/// #[test]
/// fn the_test(injected: Injected) {
///     assert_eq!(42, *injected)
/// }
/// #[main]
/// fn main() {}
/// ```
///
/// ```
/// use rustest::*;
///
/// # #[fixture]
/// # fn Injected() -> i32 { 42 }
/// #[rustest::test]
/// fn the_test(injected: Injected) {
///     assert_eq!(42, *injected)
/// }
/// #[main]
/// fn main() {}
/// ```
///
/// Your annotated function's arguments can be
/// [injected](#injecting-fixtures) with [`[fixture]`](macro@fixture)s
/// or by providing [param values](#parametrized-values).
///
/// `test` attribute can be applied to a test function.
///
/// Your test function can use take fixtures as argument and can return results.
/// They can also be marked by `#[xfail]` attribute.
///
/// In your test function you can:
///
/// - [injecting fixtures](#injecting-fixtures)
/// - Generate [parametrized test cases](#parametrized-values)
///
/// Additional Attributes:
///
/// - Function Attributes:
///   - [`#[xfail]`](#falling-tests) Expect the test to fail
///
/// ## Injecting Fixtures
///
/// The simplest case is write a test that can be injected with
/// [`[fixture]`](macro@fixture)s. You can just declare all used fixtures by passing
/// them as a function's arguments. This can help your test to be neat
/// and make your dependency clear.
///
/// ```
/// use rustest::{test, *};
///
/// #[fixture]
/// fn Injected() -> i32 { 42 }
///
/// #[test]
/// fn the_test(injected: Injected) {
///     assert_eq!(42, *injected)
/// }
///
/// #[main]
/// fn main() {}
/// ```
///
/// See [macro@fixture] macro documentation for full options on fixtures declaration.
///
/// ## Parametrized Values
///
/// You can directly provide a list of params to avoid declaring a fixture.
///
/// ```
/// use rustest::{test, *};
///
/// #[test(params:u32=[1,2,5])]
/// fn test(param: Param) {
///     assert!([1,2,5].contains(&param))
/// }
///
/// #[main]
/// fn main() {}
/// ```
///
///```
/// use rustest::test;
///
/// #[test(params:(u32, u32)=[
///     (0, 0),
///     (1, 1),
///     (2, 1),
///     (3, 2),
///     (4, 3),
///     (5, 5),
///     (6, 8),
/// ])]
/// fn fibonacci_test(param: Param) {
///     let (input, expected) = *param;
///     assert_eq!(expected, fibonacci(input))
/// }
///
/// fn fibonacci(input: u32) -> u32 {
///     match input {
///         0 => 0,
///         1 => 1,
///         n => fibonacci(n - 2) + fibonacci(n - 1)
///     }
/// }
/// #[rustest::main]
/// fn main() {}
/// ```
/// `rustest` will produce 3 independent tests and not just one that
/// check every case. Every test can fail independently and `cargo test`
/// will give follow output:
///
/// ```text
/// running 5 tests
/// test test::1 ... ok
/// test test::2 ... ok
/// test test::5 ... ok
///
/// test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
/// ```
///
/// The cases input values can be arbitrary Rust expressions that return the
/// a iterable on the type of the fixtureParam (u32 here).
///
pub use rustest_macro::test;

/// Replace a empty main function into a test harness.
///
/// ```
/// use rustest::main;
/// #[main]
/// fn main() {}
/// ```
///
/// Content of the main function is discarded. However you should not have one as it may change in
/// the future (pre or post run)
pub use rustest_macro::main;
