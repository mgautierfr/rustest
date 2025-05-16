use super::fixture::FixtureBuilder;
use libtest_mimic::Failed;
use std::{error::Error, fmt::Display};

/// Result of a test.
pub type Result = std::result::Result<(), Box<dyn Error>>;

#[doc(hidden)]
pub struct InnerTestError {
    msg: String,
}

impl InnerTestError {
    fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }
}

impl Display for InnerTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[cfg(feature = "googletest")]
impl From<googletest::internal::test_outcome::TestFailure> for InnerTestError {
    fn from(e: googletest::internal::test_outcome::TestFailure) -> Self {
        Self { msg: e.to_string() }
    }
}

/// The result of test runned by rustest.
///
/// InnerTestResult is necessary as we are somehow between user (`Result`) and libtest_mimic (`LibTestResult`).
/// In the same time, we want to integrate with googletest and it needs us to be `Display`.
/// User test is returning a Result and is converted to InnerTestResult with IntoError
/// trait.
pub type InnerTestResult = std::result::Result<(), InnerTestError>;

/// The result of test runned by libtest_mimic.
///
/// LibTestResult is necessary as it is what expected by libtest_mimic.
/// User test is returning a Result and is converted to InnerTestResult with IntoError
/// trait.
pub type LibTestResult = std::result::Result<(), Failed>;

use super::{FixtureCreationResult, FixtureRegistry, FixtureScope};
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
        self.map(|_v| ())
            .map_err(|e| InnerTestError::new(e.to_string()))
    }
}

#[cfg(feature = "googletest")]
impl<T> IntoError for googletest::Result<T> {
    fn into_error(self) -> InnerTestResult {
        self.map(|_v| ())
            .map_err(|e| InnerTestError::new(e.to_string()))
    }
}

pub type TestRunner = dyn FnOnce() -> InnerTestResult + std::panic::UnwindSafe + 'static;
pub type TestGenerator = dyn FnOnce() -> FixtureCreationResult<Box<TestRunner>> + Send + 'static;

/// An actual test run by rustest
pub struct Test {
    name: String,
    runner: Box<TestGenerator>,
    xfail: bool,
    ignore: bool,
}

fn setup_gtest() {
    #[cfg(feature = "googletest")]
    {
        use googletest::internal::test_outcome::TestOutcome;
        TestOutcome::init_current_test_outcome();
    }
}

fn collect_gtest(test_result: InnerTestResult) -> InnerTestResult {
    #[cfg(not(feature = "googletest"))]
    {
        test_result
    }

    #[cfg(feature = "googletest")]
    {
        use googletest::internal::test_outcome::TestOutcome;
        TestOutcome::close_current_test_outcome(test_result).map_err(|e| e.into())
    }
}

impl Test {
    /// Build a new test.
    pub fn new(
        name: impl Into<String>,
        xfail: bool,
        ignore: bool,
        runner: Box<TestGenerator>,
    ) -> Self {
        Self {
            name: name.into(),
            xfail,
            ignore,
            runner,
        }
    }
    fn run(self) -> LibTestResult {
        setup_gtest();
        let test_runner = (self.runner)()
            .map_err(|e| Failed::from(format!("Fixture {} error: {}", e.fixture_name, e.error)))?;
        let unwind_result = std::panic::catch_unwind(test_runner);
        let test_result = match unwind_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(cause) => {
                // We expect the cause payload to be a string or 'str
                let payload = cause
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| cause.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or(format!("{:?}", cause));
                Err(InnerTestError::new(payload))
            }
        };
        let test_result = collect_gtest(test_result);
        if self.xfail {
            match test_result {
                Ok(_) => Err("Test should fail".into()),
                Err(_) => Ok(()),
            }
        } else {
            Ok(test_result?)
        }
    }
}

impl From<Test> for libtest_mimic::Trial {
    fn from(test: Test) -> Self {
        let xfail = test.xfail;
        let ignore = test.ignore;
        let mimic_test = Self::test(test.name.clone(), move || test.run());

        let mimic_test = if xfail {
            mimic_test.with_kind("XFAIL")
        } else {
            mimic_test
        };

        mimic_test.with_ignored_flag(ignore)
    }
}

/// The context of a specific test.
///
/// Test context is mainly used to store existing fixture builder when fixture scope is Test or Global.
pub struct TestContext<'a> {
    global_reg: &'a mut FixtureRegistry,
    reg: &'a mut FixtureRegistry,
}

impl<'a> TestContext<'a> {
    pub(crate) fn new(global_reg: &'a mut FixtureRegistry, reg: &'a mut FixtureRegistry) -> Self {
        Self { global_reg, reg }
    }
    pub fn add<B>(&mut self, value: Vec<B>)
    where
        B: FixtureBuilder + 'static,
    {
        let reg = match B::SCOPE {
            FixtureScope::Test => &mut self.reg,
            FixtureScope::Global => &mut self.global_reg,
            FixtureScope::Unique => return,
        };
        reg.add::<B>(value)
    }

    pub fn get<B>(&mut self) -> Option<Vec<B>>
    where
        B: FixtureBuilder + 'static,
    {
        let reg = match B::SCOPE {
            FixtureScope::Test => &mut self.reg,
            FixtureScope::Global => &mut self.global_reg,
            FixtureScope::Unique => return None,
        };
        reg.get::<B>()
    }

    pub fn get_fixture<Fix>(&mut self) -> Vec<Fix>
    where
        Fix: FixtureBuilder + Any,
    {
        Fix::setup(self)
    }
}
