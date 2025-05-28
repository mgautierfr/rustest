use super::{
    fixture_matrix::{BuilderCall, BuilderCombination, CallArgs, Duplicate},
    test::TestContext,
    test_name::TestName,
};
use std::{
    any::{Any, TypeId},
    default::Default,
    ops::Deref,
    sync::Arc,
};

/// Represents an error that occurs during the creation of a fixture.
#[derive(Debug, Clone)]
pub struct FixtureCreationError {
    pub fixture_name: String,
    pub error: Arc<dyn std::error::Error + Sync + Send>,
}

/// The result of a fixture creation.
pub type FixtureCreationResult<T> = Result<T, FixtureCreationError>;

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
        T: std::error::Error + Sync + Send + 'static,
    {
        Self {
            fixture_name: fixture_name.into(),
            error: Arc::new(error),
        }
    }
}

/// A trait representing a [Fixture] builder.
///
///
pub trait FixtureBuilder: Duplicate + TestName {
    type Fixt: Fixture;

    const SCOPE: FixtureScope;

    /// Sets up the builders.
    ///
    /// Each builder is responsible to create a fixture.
    /// When a fixtures is parametrized (either directly or because of its dependencies),
    /// `setup` must returns as many builders as there is fixtures to build.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The test context used for setting up the fixture.
    ///
    /// # Returns
    ///
    /// A result containing a vector of builders.
    fn setup(ctx: &mut TestContext) -> Vec<Self>
    where
        Self: Sized;

    /// Build a fixture.
    ///
    /// Note that duplicated builder must build the **SAME** fixture.
    /// It is up to the builder implementation to take care of needed cache or shared states.
    fn build(self) -> FixtureCreationResult<Self::Fixt>
    where
        Self: Sized;
}

/// A trait representing a fixture that can be set up and torn down.
///
/// This trait is automatically impl by fixtures defined with [macro@crate::fixture] attribute macro.
/// You should not have to impl it.
pub trait Fixture: Deref<Target = Self::Type> {
    /// The user type of the fixture.
    type Type;
    type Builder: FixtureBuilder<Fixt = Self>;
}

/// A fixture that can be used as dependency for another fixture.
///
/// This is mainly a static Fixture. This trait is defined as syntaxic suggar to allow :
/// ```rust
/// # use rustest::{fixture, SubFixture};
/// #[fixture]
/// fn MyFixture<F>(fixt: F) -> u32
///     where F: SubFixture<Type = u32>
/// {
///     *fixt
/// }
/// ```
///
/// instead of
///
/// ```rust
/// # use rustest::{fixture, Fixture};
/// #[fixture]
/// fn MyFixture<F>(fixt: F) -> u32
///     where F: Fixture<Type = u32> + 'static
/// {
///     *fixt
/// }
/// ```
pub trait SubFixture: Fixture + 'static {}

impl<F> SubFixture for F where F: Fixture + 'static {}

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
            builder.duplicate()
        })
    }
}

/// A type alias for a teardown function.
///
/// The teardown function is called when the fixture is dropped to clean up resources.
pub type TeardownFn<T> = dyn Fn(&mut T) + Send + Sync;

/// A struct that manages the teardown of a fixture.
///
/// `FixtureTeardown` holds a value and an optional teardown function that is called when the
/// fixture is dropped.
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

#[doc(hidden)]
pub enum LazyValue<V, B> {
    Value(V),
    Builders(Option<BuilderCombination<B>>),
}

impl<V, B> From<BuilderCombination<B>> for LazyValue<V, B> {
    fn from(b: BuilderCombination<B>) -> Self {
        Self::Builders(Some(b))
    }
}

impl<V: Clone, B> LazyValue<V, B> {
    pub fn get<F, T>(&mut self, f: F) -> FixtureCreationResult<V>
    where
        F: Fn(CallArgs<T>) -> FixtureCreationResult<V>,
        BuilderCombination<B>: BuilderCall<T>,
    {
        if let LazyValue::Builders(b) = self {
            let value = b.take().unwrap().call(f)?;
            *self = LazyValue::Value(value);
        };

        match self {
            LazyValue::Value(v) => Ok(v.clone()),
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
