use core::any::{Any, TypeId};

pub use libtest_mimic;
pub use libtest_mimic::Failed;
pub use rustest_macro::{fixture, main, test};
use std::error::Error;

pub type Result = std::result::Result<(), TestError>;
type InnerTestResult = std::result::Result<(), Failed>;

#[derive(Debug)]
pub struct FixtureCreationError {
    pub fixture_name: String,
    pub error: Box<dyn std::error::Error>,
}

impl FixtureCreationError {
    pub fn new<T>(fixture_name: &str, error: T) -> Self
    where
        T: std::error::Error + 'static,
    {
        Self {
            fixture_name: fixture_name.into(),
            error: Box::new(error),
        }
    }
}

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

pub trait Fixture {
    fn setup(ctx: &mut Context) -> std::result::Result<Self, FixtureCreationError>
    where
        Self: Sized;
}

pub struct Context {
    pub fixtures: std::collections::HashMap<TypeId, Option<Box<dyn Any>>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            fixtures: Default::default(),
        }
    }

    pub fn register_fixture(&mut self, id: TypeId) {
        self.fixtures.insert(id, None);
    }

    pub fn get_fixture<T>(&mut self) -> std::result::Result<T, FixtureCreationError>
    where
        T: Fixture + Any,
    {
        T::setup(self)
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
