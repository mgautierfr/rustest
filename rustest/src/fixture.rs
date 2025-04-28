use super::TestContext;
use crate::FixtureDisplay;
use core::{
    any::{Any, TypeId},
    clone::Clone,
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
pub trait FixtureBuilder: Clone + FixtureDisplay {
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

    fn build(&self) -> Self::Fixt
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

pub trait SubFixture: Fixture + Clone + 'static {}

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
    pub(crate) fn add<F>(&mut self, value: Vec<F::InnerType>)
    where
        F: FixtureBuilder + 'static,
        F::InnerType: Clone + 'static,
    {
        self.fixtures.insert(TypeId::of::<F>(), Box::new(value));
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
    pub(crate) fn get<F>(&mut self) -> Option<Vec<F::InnerType>>
    where
        F: FixtureBuilder + 'static,
        F::InnerType: Clone + 'static,
    {
        self.fixtures.get(&TypeId::of::<F>()).map(|a| {
            let fixture = a.downcast_ref::<Vec<F::InnerType>>().unwrap();
            fixture.clone()
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

impl<T: FixtureDisplay> FixtureDisplay for FixtureTeardown<T> {
    fn display(&self) -> String {
        self.value.display()
    }
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

/// A shared fixture value that manages the teardown of a fixture.
///
/// `SharedFixtureValue` wraps a `FixtureTeardown` in an `Arc` to allow shared ownership.
#[repr(transparent)]
#[doc(hidden)]
pub struct SharedFixtureValue<T>(Arc<FixtureTeardown<T>>);

impl<T> Clone for SharedFixtureValue<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: 'static> SharedFixtureValue<T> {
    /// Builds a shared fixture value.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The test context used for building the fixture.
    /// * `f` - A builder function that returns a result containing a vector of the inner type.
    /// * `teardown` - An optional teardown function.
    ///
    /// # Returns
    ///
    /// A result containing a vector of `SharedFixtureValue` or a `FixtureCreationError`.
    pub fn build<Fx, Builder>(
        ctx: &mut TestContext,
        f: Builder,
        teardown: Option<Arc<TeardownFn<T>>>,
    ) -> std::result::Result<Vec<Self>, FixtureCreationError>
    where
        Fx: FixtureBuilder<InnerType = Self> + 'static,
        Builder: Fn(&mut TestContext) -> std::result::Result<Vec<T>, FixtureCreationError>,
    {
        if let Some(f) = ctx.get::<Fx>() {
            return Ok(f);
        }
        let values = f(ctx)?
            .into_iter()
            .map(|fix| {
                SharedFixtureValue(Arc::new(FixtureTeardown {
                    value: fix,
                    teardown: teardown.clone(),
                }))
            })
            .collect::<Vec<_>>();

        ctx.add::<Fx>(values.clone());
        Ok(values)
    }
}

impl<T> std::ops::Deref for SharedFixtureValue<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: FixtureDisplay> FixtureDisplay for SharedFixtureValue<T> {
    fn display(&self) -> String {
        self.0.display()
    }
}
