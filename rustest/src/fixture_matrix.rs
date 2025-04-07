use core::{
    clone::Clone,
    cmp::PartialEq,
    panic::{RefUnwindSafe, UnwindSafe},
};

use super::FixtureDisplay;

#[derive(Clone, Debug)]
pub struct FixtureCombination<KnownType>(KnownType);

impl<KnownType> PartialEq<KnownType> for FixtureCombination<KnownType>
where
    KnownType: PartialEq,
{
    fn eq(&self, other: &KnownType) -> bool {
        return self.0.eq(other);
    }
}

/// A matrix of fixtures.
///
/// `FixtureMatrix` is used to manage a collection of fixtures.
/// It acts as an increasing matrix of dimension N as we feed it with new fixtures vector.
#[derive(Default)]
pub struct FixtureMatrix<KnownTypes> {
    fixtures: Vec<FixtureCombination<KnownTypes>>,
    multiple: bool,
}

impl<T> FixtureMatrix<T> {
    /// Does the FixtureMatrix as multiple combination ?
    pub fn is_multiple(&self) -> bool {
        self.multiple
    }
}

impl FixtureMatrix<()> {
    /// Creates a new `FixtureMatrix` with 0 dimension.
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
            multiple: false,
        }
    }
}

impl FixtureMatrix<()> {
    /// Feeds new fixtures into the matrix.
    ///
    /// # Arguments
    ///
    /// * `new_fixs` - A vector of new fixtures to feed into the matrix.
    ///
    /// # Returns
    ///
    /// A new `FixtureMatrix` of dimension 1 containing the fed fixtures.
    pub fn feed<T: Clone>(self, new_fixs: Vec<T>) -> FixtureMatrix<(T,)> {
        let multiple = self.multiple || new_fixs.len() > 1;
        let fixtures = new_fixs
            .iter()
            .map(move |new| (FixtureCombination((new.clone(),))))
            .collect::<Vec<_>>();
        FixtureMatrix { fixtures, multiple }
    }

    /// Call the function f... with no fixture as this FixtureMatrix is dimension 0.
    pub fn call<F, Output>(self, f: F) -> impl Iterator<Item = Output>
    where
        F: Fn(String) -> Output,
    {
        vec![(f("".into()))].into_iter()
    }
}

macro_rules! impl_multiple_fixture_stuff {
    (($($types:tt),+), ($($names:ident),+)) => {

        impl< $($types),+ > FixtureDisplay for FixtureCombination<($($types),+,)>
           where
                $($types : FixtureDisplay),+ ,
        {
            fn display(&self) -> String {
                let ($($names),+, ) = &self.0;
                $(let $names = $names.display();)+
                let vec = vec![$($names),+];
                format!("[{}]", vec.join("|"))
            }
        }

        impl<$($types),+> FixtureMatrix<($($types),+,)> where
            $($types : Clone + Send + UnwindSafe + FixtureDisplay + 'static),+ ,
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
                        let name = fix.display();
                        let FixtureCombination(($($names),+, )) = fix;
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
                                let FixtureCombination(($($names),+, )) = existing.clone();
                                FixtureCombination(($($names),+ , new.clone()))
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

#[cfg(test)]
mod tests {
    use core::unimplemented;

    use super::*;
    use crate::{Fixture, FixtureCreationError, FixtureRegistry, FixtureScope, TestContext};

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
    impl<T> std::ops::Deref for DummyFixture<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<T: std::fmt::Display> FixtureDisplay for DummyFixture<T> {
        fn display(&self) -> String {
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
