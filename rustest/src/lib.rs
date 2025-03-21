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

pub trait CollectError {
    fn collect_error(self) -> InnerTestResult;
}

impl CollectError for () {
    fn collect_error(self) -> InnerTestResult {
        Ok(self)
    }
}

impl CollectError for std::result::Result<(), TestError> {
    fn collect_error(self) -> InnerTestResult {
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
