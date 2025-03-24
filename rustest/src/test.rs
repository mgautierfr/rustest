use libtest_mimic::Failed;
use std::error::Error;

pub type Result = std::result::Result<(), TestError>;
type InnerTestResult = std::result::Result<(), Failed>;

#[derive(Debug)]
pub struct TestError(pub Box<dyn Error>);

impl<T> From<T> for TestError
where
    T: std::error::Error + 'static,
{
    fn from(e: T) -> Self {
        Self(Box::new(e))
    }
}

pub trait IntoError {
    fn into_error(self) -> InnerTestResult;
}

impl IntoError for () {
    fn into_error(self) -> InnerTestResult {
        Ok(self)
    }
}

impl IntoError for std::result::Result<(), TestError> {
    fn into_error(self) -> InnerTestResult {
        self.map(|_v| ()).map_err(|e| e.0.to_string().into())
    }
}

pub struct Test {
    name: String,
    runner: Box<dyn FnOnce() -> InnerTestResult + Send>,
    xfail: bool,
}

impl Test {
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
        let test_result =
            match ::std::panic::catch_unwind(std::panic::AssertUnwindSafe(self.runner)) {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(cause) => {
                    // We expect the cause payload to be a string or 'str
                    let payload = cause
                        .downcast_ref::<String>()
                        .map(|s| s.clone())
                        .or(cause.downcast_ref::<&str>().map(|s| s.to_string()))
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
