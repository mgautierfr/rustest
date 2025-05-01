#![allow(clippy::test_attr_in_doctest)]
//! rustest, an advance test harness.
//!
//! This crate provides mainly three macros ([fixture], [test] and [main]) to set up your tests and their dependencies.
//!
//! ```
//! use rustest::{test, *};
//!
//! #[fixture]
//! fn SomeInput() -> u32 {
//!     42
//! }
//!
//! #[test]
//! fn test_42(input: SomeInput) {
//!     assert_eq!(*input, 42);
//! }
//!
//! #[main]
//! fn main() {}
//! ```
//!
//! # Setup
//!
//! Add rustest to your `Cargo.toml` file:
//!
//! ```shell
//! $ cargo add --dev rustest
//! ```
//!
//! Rustest comes with its own test harness, so you must deactivate the default one in Cargo.toml:
//!
//! ```toml
//! # In Cargo.toml
//!
//! [[test]]
//! name = "test_name" # for a test located at "tests/test_name.rs"
//! harness = false
//!
//! [[test]]
//! name = "other_test" # for a test located at "tests/other_test.rs"
//! harness = false
//!
//! # For unit test, you also need to deactivate harness for lib
//! [lib]
//! harness = false
//! ```
//!
//! You also need to add a main function in each of your integration tests. To do so add an empty main function and
//! mark it with `#[rustest::main]` attribute:
//!
//! ```rust
//! #[rustest::main]
//! fn main () {}
//! ```
//!
//! For unit testing, add the main function at end of you `lib.rs` file, but add a `cfg(test)` to add it only for tests:
//!
//! ```nocompile
//! #[cfg(test)]
//! #[rustest::main]
//! fn main() {}
//! ```
//!
//! # Feature flags
//!
//! * **googletest**: Add support for [googletest](https://crates.io/crates/googletest) matchers. See [Using google test](#using-google-test) section.
//!
//! # Using google test
//!
//! If feature flag `googletest` is activated, you can use googletest matchers. You don't need to mark you tests with `#[gtest]`.
//!
//! ```
//! # #[cfg(feature = "googletest")]
//! # mod test {
//! use googletest::prelude::*;
//! use rustest::{test, *};
//!
//! #[fixture]
//! fn Value() -> u32 { 2 }
//!
//! #[test]
//! fn succeed(value: Value) {
//!     assert_that!(*value, eq(2));
//! }
//!
//! #[test]
//! #[xfail]
//! fn fails_and_panics(value: Value) {
//!     assert_that!(*value, eq(4));
//! }
//!
//! #[test]
//! #[xfail]
//! fn two_logged_failures(value: Value) {
//!     expect_that!(*value, eq(4)); // Test now failed, but continues executing.
//!     expect_that!(*value, eq(5)); // Second failure is also logged.
//! }
//!
//! #[test]
//! #[xfail]
//! fn fails_immediately_without_panic(value: Value) -> googletest::Result<()> {
//!     verify_that!(*value, eq(4))?; // Test fails and aborts.
//!     verify_that!(*value, eq(2))?; // Never executes.
//!     Ok(())
//! }
//!
//! #[test]
//! #[xfail]
//! fn simple_assertion(value: Value) -> googletest::Result<()> {
//!     verify_that!(*value, eq(4)) // One can also just return the last assertion.
//! }
//! # }
//!
//! #[rustest::main]
//! fn main () {}
//! ```

mod fixture;
mod fixture_builder;
mod fixture_display;
mod fixture_matrix;
mod test;
use fixture::FixtureRegistry;
#[doc(hidden)]
pub use fixture::SharedFixtureValue;
pub use fixture::{
    BuildableFixture, Fixture, FixtureBuilder, FixtureCreationError, FixtureScope, LazyValue,
    SubFixture, TeardownFn,
};
pub use fixture_builder::{Builder, FixtureDef};
#[doc(hidden)]
pub use fixture_display::FixtureDisplay;
pub use fixture_matrix::{BuilderCall, BuilderCombination, CallArgs, FixtureMatrix};
#[doc(hidden)]
pub use test::{InnerTestResult, IntoError, TestGenerator, TestRunner};
pub use test::{Result, Test, TestContext};

pub use ctor::declarative::ctor;

/// Function creating a set of [Test] from a [TestContext].
///
/// Classic function will resolve fixture matrix and generate a [Test] per combination.
/// Such functions are implement by [test]
pub type TestGeneratorFn =
    fn(&mut TestContext) -> ::std::result::Result<Vec<Test>, FixtureCreationError>;

/// Build tests from `test_ctors` and run them.
///
/// You should not directly call it directly.
/// Use [main] attribute on an empty main function.
pub fn run_tests(test_generators: &[TestGeneratorFn]) -> std::process::ExitCode {
    use libtest_mimic::{Arguments, run};
    let args = Arguments::from_args();

    let mut global_registry = FixtureRegistry::new();

    let tests: ::std::result::Result<Vec<_>, FixtureCreationError> = test_generators
        .iter()
        .map(|test_generator| {
            let mut test_registry = FixtureRegistry::new();
            let mut ctx = TestContext::new(&mut global_registry, &mut test_registry);
            test_generator(&mut ctx)
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

/// Define a fixture that you can use in all `rustest`'s test and fixture arguments.
///
/// While not exact (not even valid rust), you can think of
///
/// ```nocompile
/// #[fixture(teardown=teardown_expr)]
/// fn MyFixture() -> MyType {
///     MyType::new("foo", 42)
/// }
/// ```
///
/// Being rewrited as
/// ```nocompile
/// # use std::ops::Deref;
/// # use std::{sync::Arc, marker::PhantomData};
/// # struct MyType();
/// # struct Inner<T> {_phantom: PhantomData<T>};
/// struct MyFixture(Arc<Inner<MyType>>);
///
/// impl MyFixture {
///     fn setup() -> Self {
///         MyType::new("foo", 42).into()
///     }
/// }
///
/// impl Drop for Inner<MyType> {
///     fn drop(&mut self) {
///         teardown_expr(self)
///     }
/// }
///
/// impl Deref for MyFixture {
///     type Target = MyType;
///     fn deref(&self) -> &MyType {
///         self.0.deref()
///     }
/// }
/// ```
///
/// You should just mark your function as `#[fixture]` and then use it as a test's argument.
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
/// fn AnswerToUniverse(twenty_one: TwentyOne, two: Two) -> i32 { *twenty_one * *two }
///
/// #[test]
/// fn the_test(everything: AnswerToUniverse) {
///     assert_eq!(42, *everything)
/// }
///
/// #[main]
/// fn main() {}
/// ```
///
/// Fixtures are requested by their type, ie: The name of the function marked with '#[fixture]'.
/// The name of the arguments is free.
///
///
///
/// # Fixture's scope
///
/// ## Unique scope
///
/// `#[fixture(scope=unique)]`
///
/// This is the default if not specified.
///
/// The fixture is (re)created everytime it is requested. It is not shared.
///
/// ## Test scope
///
/// `#[fixture(scope=test)]`
///
/// The fixture is created only once per test, even if the test (or its fixtures dependencies) request it
/// several times.
///
/// ## Global scope
///
/// `#[fixture(scope=global)]`
///
/// The fixture is created only once. It is shared accross all tests (in a given binary) and teardown at the end.
///
/// ```
/// use rustest::{test,*};
///
/// #[fixture(scope=global)]
/// fn GlobalFixture() -> i32 {
///     println!("Create global fixture");
///     42
/// }
///
/// #[fixture(scope=test)]
/// fn TestFixture() -> i32 {
///     println!("Create test fixture");
///     42
/// }
///
/// #[fixture]
/// fn UniqueFixture() -> i32 {
///     println!("Create unique fixture");
///     42
/// }
///
/// // Print:
/// // Create global fixture
/// // Create test fixture
/// // Create unique fixture
/// // Create unique fixture
/// #[test]
/// fn one_test(
///     global_fixture: GlobalFixture,
///     test_0: TestFixture,
///     test_1: TestFixture,
///     unique_0: UniqueFixture,
///     unique_1: UniqueFixture
/// ) {
/// }
///
/// // Print:
/// // Create test fixture
/// // Create unique fixture
/// // Create unique fixture
/// #[test]
/// fn other_test(
///     global_fixture: GlobalFixture,
///     test_0: TestFixture,
///     test_1: TestFixture,
///     unique_0: UniqueFixture,
///     unique_1: UniqueFixture
/// ) {
/// }
///
/// #[main]
/// fn main() {}
/// ```
///
/// ## Renaming
///
/// You may want to name your fixture differently than the "function" used to create it.
/// This can be done with `name=<name>` argument.
/// In this case, the name of the "function" is not used at all.
///
/// ```
/// # use rustest::{test, *};
/// #[fixture(name=MyFixture)]
/// fn setup() -> i32 { 42 }
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
/// ## Fixture Alias
///
/// As fixture are plain rust type, you can define a type alias on them:
///
/// ```
/// # use rustest::{test, *};
/// #[fixture]
/// fn MyFixture() -> i32 { 42 }
///
/// type OtherName = MyFixture;
///
/// #[test]
/// fn the_test(value: OtherName) {
///     assert_eq!(42, *value)
/// }
///
/// #[main]
/// fn main() {}
/// ```
///
/// This is particulary usefull when using partial injection.
///
/// ## Partial Injection
///
/// You may define a fixture taken another fixture as argument without knowing which exact fixture it is.
/// To do so, you create a geniric fixture.
///
/// The exact type of the fixture is determined at test level, or using a type alias.
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
/// fn Double<S: SubFixture<Type=i32>> (base: S) -> i32
/// { 2 * *base }
///
/// #[test]
/// fn test_double1(value: Double<Base1>) { assert_eq!(2, *value) }
///
/// type DoubleBase2 = Double<Base2>;
/// #[test]
/// fn test_double2(value: DoubleBase2) { assert_eq!(4, *value) }
///
/// #[main]
/// fn main() {}
/// ```
///
/// ## Parametrized
///
/// Fixture can be parametrized with the `params` argument.
///
/// The `params` argument must define the type of the parameters (`ty`) and provides a `expr` witch implement
/// `IntoIterator<Item=ty>`. Argument of the fixture's setup must be a `Param` type.
///
/// ```
/// use rustest::{test ,*};
///
/// # #[fixture]
/// # fn Double<S: SubFixture<Type=i32>> (base: S) -> i32
/// # { 2 * *base }
///
/// #[fixture(params:i32=[1,5,2])]
/// fn ParamFixture(p: Param) -> i32 { *p }
///
/// // Will run tree tests:
/// // - test_param[ParamFixture:1]
/// // - test_param[ParamFixture:5]
/// // - test_param[ParamFixture:2]
/// #[test]
/// fn test_param(value: ParamFixture) {
///     assert!([1,5,2].contains(&value))
///  }
///
/// // Will run tree tests:
/// // - test_param_double[Double:2]
/// // - test_param_double[Double:10]
/// // - test_param_double[Double:4]
/// #[test]
/// fn test_param_double(value: Double<ParamFixture>) {
///     assert!([2,10,4].contains(&value))
///  }
///
/// #[main]
/// fn main() {}
/// ```
///
/// When a fixtures is parametrized, it is part of a fixture matrix
///
/// ```
/// # use rustest::{test ,*};
/// #
/// # #[fixture]
/// # fn Double<S: SubFixture<Type=i32>> (base: S) -> i32
/// # { 2 * *base }
/// #
/// # #[fixture(params:i32=[1,5,2])]
/// # fn ParamFixture(p: Param) -> i32 { *p }
/// #
/// // Will run nine tests:
/// // - test[ParamFixture:1|Double:2]
/// // - test[ParamFixture:1|Double:10]
/// // - test[ParamFixture:1|Double:4]
/// // - test[ParamFixture:5|Double:2]
/// // - test[ParamFixture:5|Double:10]
/// // - test[ParamFixture:5|Double:4]
/// // - test[ParamFixture:2|Double:2]
/// // - test[ParamFixture:2|Double:10]
/// // - test[ParamFixture:2|Double:4]
/// #[test]
/// fn test(value0: ParamFixture, value1: Double<ParamFixture>) {
///     assert!([1,5,2].contains(&value0));
///     assert!([2,10,4].contains(&value1));
///  }
///
/// # #[main]
/// # fn main() {}
/// ```
///
/// # Fixture Teardown
///
/// Fixtures can be teardown with `teardown` argument.
///
/// ```
/// # use rustest::{test ,*};
/// #[fixture(teardown=|v| println!("Teardown with {v}"))]
/// fn TeardownFixture() -> i32 {
///     println!("Setup fixture");
///     42
/// }
///
/// // Print:
/// // ```
/// // Setup fixture
/// // Run test with 42
/// // Teardown with 42
/// // ```
/// #[test]
/// fn test(v: TeardownFixture) {
///     println!("Run test with {}", *v);
///     assert_eq!(*v, 42);
///  }
///
/// # #[main]
/// # fn main() {}
/// ```
///
/// The `teardown` value is any expression of type `Fn(&mut T)` where T is your fixture type.
///
///
/// # Fallible Fixture
///
/// By default, fixture creation should not fail.
/// If creation of the fixture may fail (io operation, ...) fixture setup can return a `Result<T, _>`.
///
/// `rustest` automatically detect if fixture creation is fallible by inspecting return type of setup function.
/// If it is a `Result` type, it assumes the fixture is faillible.
///
/// If you use a custom type of result (`MyResult`), `rustest` will not detect the fixture as fallible.
/// You can force it with `fallible=true` argument.
///
/// On the contrary, if you want you fixture to actually return a `Result` you can use `fallible=false`.
///
/// Be carreful, fixtures are created at tests collection phase. If any fixture setup fails, no tests will be run.
/// Consider fallible fixtures as a sligthly better way to handle errors than simply unwrap them but not as a full
/// error handling system.
pub use rustest_macro::fixture;

/// `test` attribute is applied to a test function.
///
/// You must explicitly use `rustest::test` (or mark the function with `#[rustest::test]`).
/// If not, rust will detect a ambigous name for `test` and refuse to compile.
///
/// ```compile_fail
/// use rustest::*;
///
/// #[test]
/// fn the_test() {
/// }
/// #[main]
/// fn main() {}
/// ```
/// ```
/// use rustest::{test,*};
///
/// #[test]
/// fn the_test() {
/// }
/// #[main]
/// fn main() {}
/// ```
///
/// ```
/// use rustest::*;
///
/// #[rustest::test]
/// fn the_test() {
/// }
/// #[main]
/// fn main() {}
/// ```
///
/// Your annotated function's arguments can be a [fixture](#injecting-fixtures) with [`[fixture]`](macro@fixture)s
/// or a [parameter](#parametrized-values).
///
/// Your test function can use fixtures as argument and can return results.
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
/// them as a function's arguments.
///
/// ```
/// use rustest::{test, *};
///
/// #[fixture]
/// fn MyFixture() -> i32 { 42 }
///
/// #[test]
/// fn the_test(v: MyFixture) {
///     assert_eq!(42, *v)
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
/// As for fixtures, you can directly provide a list of params to avoid declaring a fixture.
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
/// Param value can be destructured:
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
/// fn fibonacci_test(Param((input, expected)): Param) {
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
