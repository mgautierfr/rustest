mod fixture;
mod test;
use fixture::FixtureRegistry;
pub use fixture::{
    Fixture, FixtureCreationError, FixtureMatrix, FixtureName, FixtureParam, FixtureScope,
    SharedFixtureValue,
};
pub use libtest_mimic;
pub use libtest_mimic::Failed;
pub use rustest_macro::{fixture, main, test};
pub use test::{InnerTestResult, IntoError, Result, Test, TestContext, TestError};

pub type TestCtorFn =
    fn(&mut TestContext) -> ::std::result::Result<Vec<Test>, FixtureCreationError>;

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
