use core::{
    any::{Any, TypeId},
    clone::Clone,
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
    fn setup(ctx: &mut FixtureRegistry) -> std::result::Result<Self, FixtureCreationError>
    where
        Self: Sized;
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

    pub fn get_fixture<Fix>(&mut self) -> std::result::Result<Fix, FixtureCreationError>
    where
        Fix: Fixture + Any,
    {
        Fix::setup(self)
    }
}

#[derive(Debug)]
pub struct SharedFixtureValue<T>(Arc<T>);

impl<T> Clone for SharedFixtureValue<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: 'static> SharedFixtureValue<T> {
    pub fn build<F, Builder>(
        ctx: &mut FixtureRegistry,
        f: Builder,
    ) -> std::result::Result<Self, FixtureCreationError>
    where
        F: Fixture<InnerType = Self> + 'static,
        Builder: Fn(&mut FixtureRegistry) -> std::result::Result<T, FixtureCreationError>,
    {
        if let Some(f) = ctx.get::<F>() {
            return Ok(f);
        }
        let value = f(ctx)?.into();

        ctx.add::<F>(&value);
        Ok(value)
    }
}

impl<T> From<T> for SharedFixtureValue<T> {
    fn from(v: T) -> Self {
        Self(v.into())
    }
}

impl<T> std::ops::Deref for SharedFixtureValue<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct UniqueFixtureValue<T>(T);

impl<T> UniqueFixtureValue<T>
where
    T: Copy,
{
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Clone for UniqueFixtureValue<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Copy for UniqueFixtureValue<T> where T: Copy {}

impl<T: 'static> UniqueFixtureValue<T> {
    pub fn build<F, Builder>(
        ctx: &mut FixtureRegistry,
        f: Builder,
    ) -> std::result::Result<Self, FixtureCreationError>
    where
        F: Fixture<InnerType = Self> + 'static,
        Builder: Fn(&mut FixtureRegistry) -> std::result::Result<T, FixtureCreationError>,
    {
        let value = f(ctx)?.into();
        Ok(value)
    }
}

impl<T> From<T> for UniqueFixtureValue<T> {
    fn from(v: T) -> Self {
        Self(v)
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
