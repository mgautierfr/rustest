use super::TestContext;
use crate::{BuilderCall, BuilderCombination, CallArgs, FixtureDisplay};
use core::{
    any::{Any, TypeId},
    clone::Clone,
    default::Default,
    ops::Deref,
    panic::{RefUnwindSafe, UnwindSafe},
};
use std::sync::Arc;

/// Represents an error that occurs during the creation of a fixture.
#[derive(Debug)]
pub struct FixtureCreationError {
    pub fixture_name: String,
    pub error: Box<dyn std::error::Error>,
}

impl FixtureCreationError {
    /// Creates a new `FixtureCreationError`.
    ///
    /// # Arguments
    ///
    /// * `fixture_name` - The name of the fixture that encountered an error.
    /// * `error` - The error that occurred.
    ///
    /// # Returns
    ///
    /// A new instance of `FixtureCreationError`.
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

/// A trait representing a fixture that can be set up and torn down.
///
/// This trait is automatically impl by fixtures defined with [macro@crate::fixture] attribute macro.
/// You should not have to impl it.
pub trait FixtureBuilder: std::fmt::Debug + Clone + FixtureDisplay {
    #[doc(hidden)]
    type InnerType;

    /// The user type of the fixture.
    type Type;

    type Fixt: Fixture;

    /// Sets up the fixture and returns a result containing a vector of fixtures.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The test context used for setting up the fixture.
    ///
    /// # Returns
    ///
    /// A result containing a vector of fixtures or a `FixtureCreationError`.
    fn setup(ctx: &mut TestContext) -> std::result::Result<Vec<Self>, FixtureCreationError>
    where
        Self: Sized;

    fn build(&self) -> std::result::Result<Self::Fixt, FixtureCreationError>
    where
        Self: Sized;

    /// Returns the scope of the fixture.
    ///
    /// # Returns
    ///
    /// The scope of the fixture.
    fn scope() -> FixtureScope;
}

pub trait Fixture: Deref<Target = Self::Type> {
    /// The user type of the fixture.
    type Type;
    type Builder: FixtureBuilder<Fixt = Self>;
}

pub trait SubFixture: Fixture + Clone + std::fmt::Debug + 'static {}

/// Represents the scope of a fixture.
///
/// The scope determines the test's "lifetime" of the fixture.
pub enum FixtureScope {
    /// Fixture is used only once.
    ///
    /// The fixture is (re)created everytime we request it.
    Unique,

    /// Fixture is associated to a test.
    ///
    /// The fixture is (re)created for every tests but created only once per test.
    /// This is usefull if the test (or its fixtures' dependencies) reuse the same fixture twice.
    Test,

    /// Fixture is global for each test
    ///
    /// The fixture is created only once and teardown at end of the tests run.
    Global,
}

/// A registry for managing fixtures.
///
/// The `FixtureRegistry` is used to store and manage fixtures. It allows adding and retrieving
/// fixtures by their type.
#[derive(Default)]
pub(crate) struct FixtureRegistry {
    pub fixtures: std::collections::HashMap<TypeId, Box<dyn Any>>,
}

impl FixtureRegistry {
    /// Creates a new `FixtureRegistry`.
    ///
    /// # Returns
    ///
    /// A new instance of `FixtureRegistry`.
    pub(crate) fn new() -> Self {
        Self {
            fixtures: Default::default(),
        }
    }

    /// Adds a fixture to the registry.
    ///
    /// # Arguments
    ///
    /// * `value` - A vector of the inner type of the fixture to be added.
    ///
    /// # Type Parameters
    ///
    /// * `F` - The type of the fixture.
    pub(crate) fn add<B>(&mut self, value: Vec<B>)
    where
        B: FixtureBuilder + 'static,
    {
        self.fixtures.insert(TypeId::of::<B>(), Box::new(value));
    }

    /// Retrieves a fixture from the registry.
    ///
    /// # Arguments
    ///
    /// * `F` - The type of the fixture to retrieve.
    ///
    /// # Returns
    ///
    /// An option containing a vector of the inner type of the fixture, if found.
    pub(crate) fn get<B>(&mut self) -> Option<Vec<B>>
    where
        B: FixtureBuilder + 'static,
    {
        self.fixtures.get(&TypeId::of::<B>()).map(|a| {
            let builder = a.downcast_ref::<Vec<B>>().unwrap();
            builder.clone()
        })
    }
}

/// A type alias for a teardown function.
///
/// The teardown function is called when the fixture is dropped to clean up resources.
type TeardownFn<T> = dyn Fn(&mut T) + Send + RefUnwindSafe + UnwindSafe + Sync;

/// A struct that manages the teardown of a fixture.
///
/// `FixtureTeardown` holds a value and an optional teardown function that is called when the
/// fixture is dropped.
#[derive(Clone)]
struct FixtureTeardown<T> {
    value: T,
    teardown: Option<Arc<TeardownFn<T>>>,
}

impl<T> std::ops::Deref for FixtureTeardown<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> Drop for FixtureTeardown<T> {
    fn drop(&mut self) {
        if let Some(t) = self.teardown.take() {
            t(&mut self.value)
        }
    }
}

#[derive(Clone, Debug)]
pub enum LazyValue<V: std::fmt::Debug, B: std::fmt::Debug> {
    Value(V),
    Builders(Option<BuilderCombination<B>>),
}

impl<V: std::fmt::Debug, B: std::fmt::Debug> From<BuilderCombination<B>> for LazyValue<V, B> {
    fn from(b: BuilderCombination<B>) -> Self {
        Self::Builders(Some(b))
    }
}

impl<V: std::fmt::Debug, B: std::fmt::Debug> LazyValue<V, B> {
    pub fn get<F, T>(&mut self, f: F) -> Result<&V, FixtureCreationError>
    where
        F: Fn(Option<String>, CallArgs<T>) -> Result<V, FixtureCreationError>,
        BuilderCombination<B>: BuilderCall<T>,
    {
        if let LazyValue::Builders(b) = self {
            let value = b.take().unwrap().call(f)?;
            *self = LazyValue::Value(value);
        };

        match self {
            LazyValue::Value(v) => Ok(v),
            LazyValue::Builders(_) => unreachable!(),
        }
    }
}

/// A shared fixture value that manages the teardown of a fixture.
///
/// `SharedFixtureValue` wraps a `FixtureTeardown` in an `Arc` to allow shared ownership.
#[repr(transparent)]
#[doc(hidden)]
pub struct SharedFixtureValue<T>(Arc<FixtureTeardown<T>>);

impl<T: std::fmt::Debug> std::fmt::Debug for SharedFixtureValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedFixtureValue")
            .field("v", self.deref())
            .finish()
    }
}

impl<T> SharedFixtureValue<T> {
    pub fn new(value: T, teardown: Option<Arc<TeardownFn<T>>>) -> Self {
        Self(Arc::new(FixtureTeardown {
            value,
            teardown: teardown.clone(),
        }))
    }
}

impl<T> Clone for SharedFixtureValue<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> std::ops::Deref for SharedFixtureValue<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
