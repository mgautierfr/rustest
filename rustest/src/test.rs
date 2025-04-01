use libtest_mimic::Failed;
use std::error::Error;

/// Result of a test.
pub type Result = std::result::Result<(), Box<dyn Error>>;

#[doc(hidden)]
/// The result of test runned by libtest_mimic.
///
/// InnerTestResult is necessary as it is what expected by libtest_mimic.
/// User test is returning a Result and is converted to InnerTestResult with IntoError
/// trait.
pub type InnerTestResult = std::result::Result<(), Failed>;

use super::{Fixture, FixtureCreationError, FixtureRegistry, FixtureScope};
use std::any::Any;

#[doc(hidden)]
/// Convert the output of a test into a [InnerTestResult]
pub trait IntoError {
    fn into_error(self) -> InnerTestResult;
}

impl IntoError for () {
    fn into_error(self) -> InnerTestResult {
        Ok(self)
    }
}

impl IntoError for Result {
    fn into_error(self) -> InnerTestResult {
        self.map(|_v| ()).map_err(|e| e.to_string().into())
    }
}

/// An actual test run by rustest
pub struct Test {
    name: String,
    runner: Box<dyn FnOnce() -> InnerTestResult + Send + std::panic::UnwindSafe>,
    xfail: bool,
}

impl Test {
    /// Build a new test.
    pub fn new<F>(name: impl Into<String>, xfail: bool, runner: F) -> Self
    where
        F: FnOnce() -> InnerTestResult + Send + std::panic::UnwindSafe + 'static,
    {
        Self {
            name: name.into(),
            xfail,
            runner: Box::new(runner),
        }
    }
    fn run(self) -> InnerTestResult {
        let test_result = match ::std::panic::catch_unwind(self.runner) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(cause) => {
                // We expect the cause payload to be a string or 'str
                let payload = cause
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| cause.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or(format!("{:?}", cause));
                Err(payload.into())
            }
        };
        if self.xfail {
            match test_result {
                Ok(_) => Err("Test should fail".into()),
                Err(_) => Ok(()),
            }
        } else {
            test_result
        }
    }
}

impl From<Test> for libtest_mimic::Trial {
    fn from(test: Test) -> Self {
        let xfail = test.xfail;
        let mimic_test = Self::test(test.name.clone(), move || test.run());

        if xfail {
            mimic_test.with_kind("XFAIL")
        } else {
            mimic_test
        }
    }
}

/// The context of a specific test.
pub struct TestContext<'a> {
    global_reg: &'a mut FixtureRegistry,
    reg: &'a mut FixtureRegistry,
}

impl<'a> TestContext<'a> {
    pub(crate) fn new(global_reg: &'a mut FixtureRegistry, reg: &'a mut FixtureRegistry) -> Self {
        Self { global_reg, reg }
    }
    pub(crate) fn add<F>(&mut self, value: Vec<F::InnerType>)
    where
        F: Fixture + 'static,
        F::InnerType: Clone + 'static,
    {
        let reg = match F::scope() {
            FixtureScope::Test => &mut self.reg,
            FixtureScope::Global => &mut self.global_reg,
            FixtureScope::Unique => return,
        };
        reg.add::<F>(value)
    }

    pub(crate) fn get<F>(&mut self) -> Option<Vec<F::InnerType>>
    where
        F: Fixture + 'static,
        F::InnerType: Clone + 'static,
    {
        let reg = match F::scope() {
            FixtureScope::Test => &mut self.reg,
            FixtureScope::Global => &mut self.global_reg,
            FixtureScope::Unique => return None,
        };
        reg.get::<F>()
    }

    pub fn get_fixture<Fix>(&mut self) -> std::result::Result<Vec<Fix>, FixtureCreationError>
    where
        Fix: Fixture + Any,
    {
        Fix::setup(self)
    }
}
