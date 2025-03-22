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

pub fn run_test<F>(f: F, xfail: bool) -> InnerTestResult
where
    F: FnOnce() -> InnerTestResult + std::panic::UnwindSafe,
{
    let test_result = match ::std::panic::catch_unwind(f) {
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
    if xfail {
        match test_result {
            Ok(_) => Err("Test should fail".into()),
            Err(_) => Ok(()),
        }
    } else {
        test_result
    }
}
