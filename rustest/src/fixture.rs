use super::TestContext;
use core::{
    any::{Any, TypeId},
    clone::Clone,
    fmt::Debug,
    panic::{RefUnwindSafe, UnwindSafe},
};
use std::sync::Arc;

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
    type InnerType;
    fn setup(ctx: &mut TestContext) -> std::result::Result<Self, FixtureCreationError>
    where
        Self: Sized;

    fn scope() -> FixtureScope;
}

pub enum FixtureScope {
    Unique,
    Test,
    Global,
}

pub fn get_fixture<Fix>(ctx: &mut TestContext) -> std::result::Result<Fix, FixtureCreationError>
where
    Fix: Fixture + Any,
{
    ctx.get_fixture()
}

pub struct FixtureRegistry {
    pub fixtures: std::collections::HashMap<TypeId, Box<dyn Any>>,
}

impl FixtureRegistry {
    pub fn new() -> Self {
        Self {
            fixtures: Default::default(),
        }
    }

    pub fn add<F>(&mut self, value: &F::InnerType)
    where
        F: Fixture + 'static,
        F::InnerType: Clone + 'static,
    {
        self.fixtures
            .insert(std::any::TypeId::of::<F>(), Box::new(value.clone()));
    }

    pub fn get<F>(&mut self) -> Option<F::InnerType>
    where
        F: Fixture + 'static,
        F::InnerType: Clone + 'static,
    {
        self.fixtures.get(&std::any::TypeId::of::<F>()).map(|a| {
            let fixture = a.downcast_ref::<F::InnerType>().unwrap();
            fixture.clone()
        })
    }
}

pub type TeardownFn<T> = dyn FnOnce(&mut T) + Send + RefUnwindSafe + UnwindSafe + Sync;

pub struct FixtureTeardown<T> {
    value: T,
    teardown: Option<Box<TeardownFn<T>>>,
}

impl<T: Debug> Debug for FixtureTeardown<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Teardown")
            .field("value", &self.value)
            .field(
                "teardown",
                if self.teardown.is_none() {
                    &"None"
                } else {
                    &"Some(...)"
                },
            )
            .finish()
    }
}

impl<T> std::ops::Deref for FixtureTeardown<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> std::ops::DerefMut for FixtureTeardown<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Drop for FixtureTeardown<T> {
    fn drop(&mut self) {
        let teardown = self.teardown.take();
        teardown.map(|t| t(&mut self.value));
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct SharedFixtureValue<T>(Arc<FixtureTeardown<T>>);

impl<T> Clone for SharedFixtureValue<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: 'static> SharedFixtureValue<T> {
    pub fn build<Fx, Builder>(
        ctx: &mut TestContext,
        f: Builder,
        teardown: Option<Box<TeardownFn<T>>>,
    ) -> std::result::Result<Self, FixtureCreationError>
    where
        Fx: Fixture<InnerType = Self> + 'static,
        Builder: Fn(&mut TestContext) -> std::result::Result<T, FixtureCreationError>,
    {
        if let Some(f) = ctx.get::<Fx>() {
            return Ok(f);
        }
        let value = SharedFixtureValue(Arc::new(FixtureTeardown {
            value: f(ctx)?,
            teardown,
        }));

        ctx.add::<Fx>(&value);
        Ok(value)
    }
}

impl<T> std::ops::Deref for SharedFixtureValue<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct UniqueFixtureValue<T>(FixtureTeardown<T>);

impl<T: 'static> UniqueFixtureValue<T> {
    pub fn build<F, Builder>(
        ctx: &mut TestContext,
        f: Builder,
        teardown: Option<Box<TeardownFn<T>>>,
    ) -> std::result::Result<Self, FixtureCreationError>
    where
        F: Fixture<InnerType = Self> + 'static,
        Builder: Fn(&mut TestContext) -> std::result::Result<T, FixtureCreationError>,
    {
        let value = UniqueFixtureValue(FixtureTeardown {
            value: f(ctx)?,
            teardown,
        });
        Ok(value)
    }
}

impl<T> std::ops::Deref for UniqueFixtureValue<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> std::ops::DerefMut for UniqueFixtureValue<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
