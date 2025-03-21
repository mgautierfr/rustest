use core::any::{Any, TypeId};

pub use libtest_mimic;
pub use libtest_mimic::Failed;
pub use rustest_macro::{fixture, main, test};

pub type Result<T = ()> = std::result::Result<T, Failed>;

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

pub trait ToResult<T> {
    fn to_result(self) -> Result<T>
    where
        Self: Sized;
}

impl<T> ToResult<T> for Result<T> {
    fn to_result(self) -> Result<T> {
        self
    }
}

impl<T, E> ToResult<T> for Result<::std::result::Result<T, E>>
where
    E: Into<Failed>,
{
    fn to_result(self) -> Result<T> {
        self.unwrap().map_err(|e| e.into())
    }
}

pub trait CollectError<T> {
    fn collect_error(self) -> Result<T>;
}

impl<T> CollectError<T> for T {
    fn collect_error(self) -> Result<T> {
        Ok(self)
    }
}

impl<T, E> CollectError<T> for std::result::Result<T, E>
where
    E: Into<Failed>,
{
    fn collect_error(self) -> Result<T> {
        self.map_err(|e| e.into())
    }
}

pub trait Fixture: Clone {
    fn setup(ctx: &mut Context) -> std::result::Result<Self, FixtureCreationError>
    where
        Self: Sized;
}

pub struct Context {
    fixtures: std::collections::HashMap<TypeId, Option<Box<dyn Any>>>,
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
        if !self.fixtures.contains_key(&TypeId::of::<T>()) {
            return T::setup(self);
        }

        if let Some(f) = self.fixtures.get(&TypeId::of::<T>()).unwrap() {
            let fixture = f.downcast_ref::<T>().unwrap();
            return Ok(fixture.clone());
        }

        let value = T::setup(self)?;
        self.fixtures
            .insert(TypeId::of::<T>(), Some(Box::new(value.clone())));
        Ok(value)
    }
}
