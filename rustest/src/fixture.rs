use core::any::{Any, TypeId};

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

pub trait Fixture {
    fn setup(ctx: &mut Context) -> std::result::Result<Self, FixtureCreationError>
    where
        Self: Sized;
}

pub struct Context {
    pub fixtures: std::collections::HashMap<TypeId, Box<dyn Any>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            fixtures: Default::default(),
        }
    }

    pub fn get_fixture<T>(&mut self) -> std::result::Result<T, FixtureCreationError>
    where
        T: Fixture + Any,
    {
        T::setup(self)
    }
}
