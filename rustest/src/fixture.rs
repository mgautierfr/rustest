use super::TestContext;
use core::{
    any::{Any, TypeId},
    clone::Clone,
    fmt::{Debug, Display},
    ops::Deref,
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

pub trait Fixture:
    FixtureName + Deref<Target = Self::Type> + Send + UnwindSafe + Clone + 'static
{
    type InnerType;
    type Type;
    fn setup(ctx: &mut TestContext) -> std::result::Result<Vec<Self>, FixtureCreationError>
    where
        Self: Sized;

    fn scope() -> FixtureScope;
}

pub enum FixtureScope {
    Unique,
    Test,
    Global,
}

#[derive(Default)]
pub(crate) struct FixtureRegistry {
    pub fixtures: std::collections::HashMap<TypeId, Box<dyn Any>>,
}

impl FixtureRegistry {
    pub(crate) fn new() -> Self {
        Self {
            fixtures: Default::default(),
        }
    }

    pub(crate) fn add<F>(&mut self, value: Vec<F::InnerType>)
    where
        F: Fixture + 'static,
        F::InnerType: Clone + 'static,
    {
        self.fixtures.insert(TypeId::of::<F>(), Box::new(value));
    }

    pub(crate) fn get<F>(&mut self) -> Option<Vec<F::InnerType>>
    where
        F: Fixture + 'static,
        F::InnerType: Clone + 'static,
    {
        self.fixtures.get(&TypeId::of::<F>()).map(|a| {
            let fixture = a.downcast_ref::<Vec<F::InnerType>>().unwrap();
            fixture.clone()
        })
    }
}

type TeardownFn<T> = dyn Fn(&mut T) + Send + RefUnwindSafe + UnwindSafe + Sync;

#[derive(Clone)]
struct FixtureTeardown<T> {
    value: T,
    teardown: Option<Arc<TeardownFn<T>>>,
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

impl<T: Debug> FixtureName for FixtureTeardown<T> {
    fn name(&self) -> String {
        format!("{:?}", self.value)
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
        teardown: Option<Arc<TeardownFn<T>>>,
    ) -> std::result::Result<Vec<Self>, FixtureCreationError>
    where
        Fx: Fixture<InnerType = Self> + 'static,
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

impl<T: Debug> FixtureName for SharedFixtureValue<T> {
    fn name(&self) -> String {
        self.0.name()
    }
}

#[derive(Default)]
pub struct FixtureMatrix<KnownTypes> {
    fixtures: Vec<KnownTypes>,
    multiple: bool,
}

impl<T> FixtureMatrix<T> {
    pub fn is_multiple(&self) -> bool {
        self.multiple
    }
}

impl FixtureMatrix<()> {
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
            multiple: false,
        }
    }
}

impl FixtureMatrix<()> {
    pub fn feed<T: Clone>(self, new_fixs: Vec<T>) -> FixtureMatrix<((), T)> {
        let multiple = self.multiple || new_fixs.len() > 1;
        let fixtures = new_fixs
            .iter()
            .map(move |new| ((), new.clone()))
            .collect::<Vec<_>>();
        FixtureMatrix { fixtures, multiple }
    }

    pub fn call<F, Output>(self, f: F) -> impl Iterator<Item = Output>
    where
        F: Fn(String) -> Output,
    {
        vec![(f("".into()))].into_iter()
    }
}

pub trait FixtureName {
    fn name(&self) -> String;
}

macro_rules! impl_multiple_fixture_stuff {
    (($($types:tt),+), ($($names:ident),+)) => {

        impl< $($types),+ > FixtureName for ($($types),+,)
           where
                $($types : Send + UnwindSafe + FixtureName + 'static),+ ,
        {
            fn name(&self) -> String {
                let ($($names),+, ) = self;
                $(let $names = $names.name();)+
                let vec = vec![$($names),+];
                vec.join("|")
            }
        }

        impl<$($types),+> FixtureMatrix<($($types),+,)> where
            $($types : Clone + Send + UnwindSafe + FixtureName + 'static),+ ,
        {
            pub fn call<F, Output>(
                self,
                f: F,
            ) -> impl Iterator<Item = Output>
                where
                F: Fn(String, $($types),+) -> Output + Send + Sync + UnwindSafe + RefUnwindSafe + 'static,
            {
                self.fixtures
                    .into_iter()
                    .map(move |fix| {
                        let name = fix.name();
                        let ($($names),+, ) = fix;
                        f(name, $($names),+)
                    })
            }

            /// Feeds new fixtures into the matrix.
            ///
            /// # Arguments
            ///
            /// * `new_fixs` - A vector of new fixtures to feed into the matrix.
            ///
            /// # Returns
            ///
            /// A new `FixtureMatrix` containing the fed fixtures.
            pub fn feed<T: Clone>(self, new_fixs: Vec<T>) -> FixtureMatrix<($($types),+ ,T)> {
                let multiple = self.multiple || new_fixs.len() > 1;
                let fixtures = self
                    .fixtures
                    .into_iter()
                    .flat_map(|existing| {
                        new_fixs
                            .iter()
                            .map(move |new| {
                                let ($($names),+, ) = existing.clone();
                                ($($names),+ , new.clone())
                            })
                    })
                    .collect::<Vec<_>>();
                FixtureMatrix { fixtures, multiple }
            }
        }
    };
}

impl_multiple_fixture_stuff!((F0), (f0));
impl_multiple_fixture_stuff!((F0, F1), (f0, f1));
impl_multiple_fixture_stuff!((F0, F1, F2), (f0, f1, f2));
impl_multiple_fixture_stuff!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_multiple_fixture_stuff!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_multiple_fixture_stuff!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_multiple_fixture_stuff!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_multiple_fixture_stuff!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_multiple_fixture_stuff!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_multiple_fixture_stuff!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_multiple_fixture_stuff!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_multiple_fixture_stuff!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

#[derive(Clone)]
pub struct FixtureParam<T>(T);

impl<T: Display> FixtureName for FixtureParam<T> {
    fn name(&self) -> String {
        format!("{}", self.0)
    }
}

impl<T> Deref for FixtureParam<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for FixtureParam<T> {
    fn from(v: T) -> Self {
        Self(v)
    }
}

impl<T> FixtureParam<T> {
    pub fn into(self) -> T {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use core::unimplemented;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct DummyFixture<T>(T);
    impl<T> Fixture for DummyFixture<T>
    where
        T: Send + Clone + UnwindSafe + std::fmt::Display + 'static,
    {
        type Type = T;
        type InnerType = T;
        fn setup(_ctx: &mut TestContext) -> std::result::Result<Vec<Self>, FixtureCreationError>
        where
            Self: Sized,
        {
            unimplemented!()
        }

        fn scope() -> FixtureScope {
            FixtureScope::Unique
        }
    }
    impl<T> Deref for DummyFixture<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<T: std::fmt::Display> FixtureName for DummyFixture<T> {
        fn name(&self) -> String {
            format!("{}", self.0)
        }
    }

    #[test]
    fn test_empty_fixture_registry() {
        let mut registry = FixtureRegistry::new();
        assert!(registry.get::<DummyFixture<i32>>().is_none());
    }

    #[test]
    fn test_fixture_registry() {
        let mut registry = FixtureRegistry::new();
        registry.add::<DummyFixture<u32>>(vec![1u32, 2u32]);
        let fixtures = registry.get::<DummyFixture<u32>>().unwrap();
        assert_eq!(fixtures.len(), 2);
        assert_eq!(fixtures[0], 1);
        assert_eq!(fixtures[1], 2);
        assert!(registry.get::<DummyFixture<u16>>().is_none());
    }

    #[test]
    fn test_fixture_matrix() {
        let matrix = FixtureMatrix::new()
            .feed(vec![DummyFixture(1), DummyFixture(2), DummyFixture(3)])
            .feed(vec![DummyFixture("Hello"), DummyFixture("World")]);
        assert_eq!(matrix.fixtures.len(), 6);
        assert_eq!(matrix.fixtures[0], (DummyFixture(1), DummyFixture("Hello")));
        assert_eq!(matrix.fixtures[1], (DummyFixture(1), DummyFixture("World")));
        assert_eq!(matrix.fixtures[2], (DummyFixture(2), DummyFixture("Hello")));
        assert_eq!(matrix.fixtures[3], (DummyFixture(2), DummyFixture("World")));
        assert_eq!(matrix.fixtures[4], (DummyFixture(3), DummyFixture("Hello")));
        assert_eq!(matrix.fixtures[5], (DummyFixture(3), DummyFixture("World")));
    }

    #[test]
    fn test_matrix_caller() {
        let matrix =
            FixtureMatrix::new().feed(vec![DummyFixture(1), DummyFixture(2), DummyFixture(3)]);
        let matrix = matrix.feed(vec![DummyFixture("Hello"), DummyFixture("World")]);
        let results = matrix.call(|_, x, s| (*x + 1, *s));
        let mut iter = results.into_iter();
        assert_eq!(iter.next().unwrap(), (2, "Hello"));
        assert_eq!(iter.next().unwrap(), (2, "World"));
        assert_eq!(iter.next().unwrap(), (3, "Hello"));
        assert_eq!(iter.next().unwrap(), (3, "World"));
        assert_eq!(iter.next().unwrap(), (4, "Hello"));
        assert_eq!(iter.next().unwrap(), (4, "World"));
    }

    #[test]
    fn test_matrix_caller_dim3() {
        let matrix =
            FixtureMatrix::new().feed(vec![DummyFixture(1), DummyFixture(2), DummyFixture(3)]);
        let matrix = matrix.feed(vec![DummyFixture("Hello"), DummyFixture("World")]);
        let matrix = matrix.feed(vec![DummyFixture(42)]);
        let results = matrix.call(|_, x, s, y| (*x + 1, *s, *y));
        let mut iter = results.into_iter();
        assert_eq!(iter.next().unwrap(), (2, "Hello", 42));
        assert_eq!(iter.next().unwrap(), (2, "World", 42));
        assert_eq!(iter.next().unwrap(), (3, "Hello", 42));
        assert_eq!(iter.next().unwrap(), (3, "World", 42));
        assert_eq!(iter.next().unwrap(), (4, "Hello", 42));
        assert_eq!(iter.next().unwrap(), (4, "World", 42));
    }
}
