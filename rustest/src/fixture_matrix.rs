use std::cmp::PartialEq;

use super::{FixtureBuilder, FixtureCreationError, TestName};

pub struct CallArgs<Types>(pub Types);

pub struct BuilderCombination<KnownType>(KnownType);

impl<KnownType> PartialEq<KnownType> for BuilderCombination<KnownType>
where
    KnownType: PartialEq,
{
    fn eq(&self, other: &KnownType) -> bool {
        self.0.eq(other)
    }
}

pub trait BuilderCall<Args> {
    fn call<F, Output>(self, f: F) -> Result<Output, FixtureCreationError>
    where
        F: FnOnce(CallArgs<Args>) -> Result<Output, FixtureCreationError>;
}

macro_rules! impl_fixture_combination_call {
    ((), ()) => {
        impl BuilderCall<()> for BuilderCombination<()> where
        {
            fn call<F, Output>(
                self,
                f: F,
            ) -> Result<Output, FixtureCreationError>
                where
                F: FnOnce(CallArgs<()>) -> Result<Output, FixtureCreationError>,
            {
                f(CallArgs(()))
            }
        }
    };
    (($($types:tt),+), ($($names:ident),+)) => {

        impl<$($types),+> BuilderCall<($($types::Fixt),+,)> for BuilderCombination<($($types),+,)> where
            $($types : FixtureBuilder + 'static),+ ,
        {
            fn call<F, Output>(
                self,
                f: F,
            ) -> Result<Output, FixtureCreationError>
                where
                F: FnOnce(CallArgs<($($types::Fixt),+,)>) -> Result<Output, FixtureCreationError>,
            {
                let ($($names),+, ) = self.0;
                let call_args = CallArgs(($($names.build()?),+,));
                f(call_args)
            }
        }
    }
}

impl_fixture_combination_call!((), ());
impl_fixture_combination_call!((F0), (f0));
impl_fixture_combination_call!((F0, F1), (f0, f1));
impl_fixture_combination_call!((F0, F1, F2), (f0, f1, f2));
impl_fixture_combination_call!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_fixture_combination_call!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_fixture_combination_call!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_fixture_combination_call!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_fixture_combination_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_fixture_combination_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_fixture_combination_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_fixture_combination_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_fixture_combination_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

/// A matrix of fixtures.
///
/// `FixtureMatrix` is used to manage a collection of fixtures.
/// It acts as an increasing matrix of dimension N as we feed it with new fixtures vector.
#[derive(Default)]
pub struct FixtureMatrix<BuildersTypes> {
    builders: BuildersTypes,
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
            builders: (),
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
    pub fn feed<T>(self, new_fixs: Vec<T>) -> FixtureMatrix<(Vec<T>,)> {
        let multiple = self.multiple || new_fixs.len() > 1;
        FixtureMatrix {
            builders: (new_fixs,),
            multiple,
        }
    }

    /// Call the function f... with no fixture as this FixtureMatrix is dimension 0.
    pub fn flatten(self) -> Vec<BuilderCombination<()>> {
        vec![BuilderCombination(())]
    }
}

macro_rules! impl_fixture_test_name {
    ((), ()) => {
        impl TestName for BuilderCombination<()>
        {
            fn name(&self) -> Option<String> {
                None
            }
        }
    };
    (($($types:tt),+), ($($names:ident),+)) => {
        impl< $($types),+ > TestName for BuilderCombination<($($types),+,)>
           where
                $($types : TestName),+ ,
        {
            fn name(&self) -> Option<String> {
                let ($($names),+, ) = &self.0;
                $(let $names = $names.name();)+
                let mut vec = vec![$($names),+].into_iter().filter_map(|d|d).collect::<Vec<_>>();
                if vec.is_empty() {
                    None
                } else if vec.len() == 1 {
                    Some(vec.pop().unwrap())
                } else {
                    Some(format!("[{}]", vec.join("|")))
                }
            }
        }
    }
}

impl_fixture_test_name!((), ());
impl_fixture_test_name!((F0), (f0));
impl_fixture_test_name!((F0, F1), (f0, f1));
impl_fixture_test_name!((F0, F1, F2), (f0, f1, f2));
impl_fixture_test_name!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_fixture_test_name!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_fixture_test_name!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_fixture_test_name!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_fixture_test_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_fixture_test_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_fixture_test_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_fixture_test_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_fixture_test_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

/// Duplicate trait is really closed to Clone trait but with slightly sementic difference.
///
/// Duplicated builders must produce the **SAME** fitxure. `Clone` trait do not provide
/// this sementics.
/// Producing the same value implies builders should have some kind of shared cache
/// (using Rc or Arc) (btw, Rc + Clone is kind of Duplicate).
pub trait Duplicate {
    /// Duplicate the value.
    fn duplicate(&self) -> Self;
}

impl<T: Duplicate> Duplicate for Vec<T> {
    fn duplicate(&self) -> Self {
        self.iter().map(|v| v.duplicate()).collect()
    }
}

macro_rules! iter_builder {
    (@call $collect:expr ; $last_name:ident ; $last_builder:expr ; ) => {
        for $last_name in $last_builder.iter() {
            let combination = BuilderCombination(($last_name.duplicate(), ));
            $collect.push(combination)
        }
    };
    (@call $collect:expr ; $last_name:ident ; $last_builder:expr ; $($known:tt),*) => {
        for $last_name in $last_builder.iter() {
            let combination = BuilderCombination(($($known.duplicate()),*, $last_name.duplicate()));
            $collect.push(combination)
        }
    };
    (@call $collect:expr ; $first_name:tt, $($other_names:ident),* ; $first_builder:expr, $($other_builders:expr),* ; ) => {
        for $first_name in $first_builder.iter() {
            iter_builder!(@call $collect ; $($other_names),* ; $($other_builders),* ; $first_name)
        }
    };
    (@call $collect:expr ; $first_name:tt, $($other_names:ident),* ; $first_builder:expr, $($other_builders:expr),* ; $($known:expr),*) => {
        for $first_name in $first_builder.iter() {
            iter_builder!(@call $collect ; $($other_names),* ; $($other_builders),* ; $($known),* , $first_name)
        }
    };
}

macro_rules! impl_fixture_call {
    (($($types:tt),+), ($($bnames:ident),+), ($($fnames:ident),+)) => {

        impl<$($types),+> FixtureMatrix<($(Vec<$types>),+,)> where
            $($types : Duplicate + TestName + 'static),+ ,
        {
            pub fn flatten(self) -> Vec<BuilderCombination<($($types),+,)>>
            {
                let ($($bnames),+, ) = self.builders;
                let mut output = vec![];
                iter_builder!(@call output ; $($fnames),+ ; $($bnames),+ ;);
                output
            }
        }
    }
}

impl_fixture_call!((F0), (b0), (f0));
impl_fixture_call!((F0, F1), (b0, b1), (f0, f1));
impl_fixture_call!((F0, F1, F2), (b0, b1, b2), (f0, f1, f2));
impl_fixture_call!((F0, F1, F2, F3), (b0, b1, b2, b3), (f0, f1, f2, f3));
impl_fixture_call!(
    (F0, F1, F2, F3, F4),
    (b0, b1, b2, b3, b4),
    (f0, f1, f2, f3, f4)
);
impl_fixture_call!(
    (F0, F1, F2, F3, F4, F5),
    (b0, b1, b2, b3, b4, b5),
    (f0, f1, f2, f3, f4, f5)
);
impl_fixture_call!(
    (F0, F1, F2, F3, F4, F5, F6),
    (b0, b1, b2, b3, b4, b5, b6),
    (f0, f1, f2, f3, f4, f5, f6)
);
impl_fixture_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (b0, b1, b2, b3, b4, b5, b6, b7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_fixture_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (b0, b1, b2, b3, b4, b5, b6, b7, b8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_fixture_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (b0, b1, b2, b3, b4, b5, b6, b7, b8, b9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_fixture_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_fixture_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

macro_rules! impl_fixture_feed {
    (($($types:tt),+), ($($names:ident),+)) => {
        impl<$($types),+> FixtureMatrix<($(Vec<$types>),+,)> where
            $($types : TestName + 'static),+ ,
        {

            /// Feeds new fixtures into the matrix.
            ///
            /// # Arguments
            ///
            /// * `new_fixs` - A vector of new fixtures to feed into the matrix.
            ///
            /// # Returns
            ///
            /// A new `FixtureMatrix` containing the fed fixtures.
            pub fn feed<T>(self, new_fixs: Vec<T>) -> FixtureMatrix<($(Vec<$types>),+ ,Vec<T>)> {
                let multiple = self.multiple || new_fixs.len() > 1;
                                let ($($names),+, ) = self.builders;
                                let builders = ($($names),+ , new_fixs);
                FixtureMatrix { builders, multiple }
            }
        }
    };
}

impl_fixture_feed!((F0), (f0));
impl_fixture_feed!((F0, F1), (f0, f1));
impl_fixture_feed!((F0, F1, F2), (f0, f1, f2));
impl_fixture_feed!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_fixture_feed!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_fixture_feed!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_fixture_feed!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_fixture_feed!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_fixture_feed!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_fixture_feed!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_fixture_feed!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_fixture_feed!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

#[cfg(test)]
mod tests {
    use core::{option::Option::None, unimplemented};

    use super::*;
    use crate::{
        Duplicate, Fixture, FixtureBuilder, FixtureCreationError, FixtureRegistry, FixtureScope,
        TestContext,
    };

    struct DummyFixture<T>(T);

    #[derive(Debug, PartialEq)]
    struct DummyFixtureBuilder<T>(T);

    impl<T: Copy> Duplicate for DummyFixtureBuilder<T> {
        fn duplicate(&self) -> Self {
            Self(self.0)
        }
    }
    impl<T> FixtureBuilder for DummyFixtureBuilder<T>
    where
        T: Send + Copy + std::fmt::Display + 'static,
    {
        type Type = T;
        type Fixt = DummyFixture<T>;
        const SCOPE: FixtureScope = FixtureScope::Unique;

        fn setup(_ctx: &mut TestContext) -> Vec<Self>
        where
            Self: Sized,
        {
            unimplemented!()
        }

        fn build(&self) -> Result<DummyFixture<T>, FixtureCreationError> {
            Ok(DummyFixture(self.0))
        }
    }

    impl<T> Fixture for DummyFixture<T>
    where
        T: Send + Copy + std::fmt::Display + 'static,
    {
        type Type = T;
        type Builder = DummyFixtureBuilder<T>;
    }
    impl<T> std::ops::Deref for DummyFixture<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<T: std::fmt::Display> TestName for DummyFixtureBuilder<T> {
        fn name(&self) -> Option<String> {
            None
        }
    }

    #[test]
    fn test_empty_fixture_registry() {
        let mut registry = FixtureRegistry::new();
        assert!(registry.get::<DummyFixtureBuilder<i32>>().is_none());
    }

    #[test]
    fn test_fixture_registry() {
        let mut registry = FixtureRegistry::new();
        registry.add(vec![DummyFixtureBuilder(1u32), DummyFixtureBuilder(2u32)]);
        let builders = registry.get::<DummyFixtureBuilder<u32>>().unwrap();
        assert_eq!(builders.len(), 2);
        assert_eq!(builders[0], DummyFixtureBuilder(1));
        assert_eq!(builders[1], DummyFixtureBuilder(2));
        assert!(registry.get::<DummyFixtureBuilder<u16>>().is_none());
    }

    #[test]
    fn test_fixture_matrix() {
        let matrix = FixtureMatrix::new()
            .feed(vec![
                DummyFixtureBuilder(1),
                DummyFixtureBuilder(2),
                DummyFixtureBuilder(3),
            ])
            .feed(vec![
                DummyFixtureBuilder("Hello"),
                DummyFixtureBuilder("World"),
            ]);
        assert_eq!(
            matrix.builders.0,
            vec![
                DummyFixtureBuilder(1),
                DummyFixtureBuilder(2),
                DummyFixtureBuilder(3)
            ]
        );
        assert_eq!(
            matrix.builders.1,
            vec![DummyFixtureBuilder("Hello"), DummyFixtureBuilder("World")]
        );
    }

    #[test]
    fn test_matrix_caller() {
        let matrix = FixtureMatrix::new().feed(vec![
            DummyFixtureBuilder(1),
            DummyFixtureBuilder(2),
            DummyFixtureBuilder(3),
        ]);
        let matrix = matrix.feed(vec![
            DummyFixtureBuilder("Hello"),
            DummyFixtureBuilder("World"),
        ]);
        let combinations = matrix.flatten();
        let results = combinations
            .into_iter()
            .map(|c| c.call(|CallArgs((x, s))| Ok((*x + 1, *s))));

        let mut iter = results.into_iter();
        assert_eq!(iter.next().unwrap().unwrap(), (2, "Hello"));
        assert_eq!(iter.next().unwrap().unwrap(), (2, "World"));
        assert_eq!(iter.next().unwrap().unwrap(), (3, "Hello"));
        assert_eq!(iter.next().unwrap().unwrap(), (3, "World"));
        assert_eq!(iter.next().unwrap().unwrap(), (4, "Hello"));
        assert_eq!(iter.next().unwrap().unwrap(), (4, "World"));
    }

    #[test]
    fn test_matrix_caller_dim3() {
        let matrix = FixtureMatrix::new().feed(vec![
            DummyFixtureBuilder(1),
            DummyFixtureBuilder(2),
            DummyFixtureBuilder(3),
        ]);
        let matrix = matrix.feed(vec![
            DummyFixtureBuilder("Hello"),
            DummyFixtureBuilder("World"),
        ]);
        let matrix = matrix.feed(vec![DummyFixtureBuilder(42)]);
        let combinations = matrix.flatten();
        let results = combinations
            .into_iter()
            .map(|c| c.call(|CallArgs((x, s, y))| Ok((*x + 1, *s, *y))));
        let mut iter = results.into_iter();
        assert_eq!(iter.next().unwrap().unwrap(), (2, "Hello", 42));
        assert_eq!(iter.next().unwrap().unwrap(), (2, "World", 42));
        assert_eq!(iter.next().unwrap().unwrap(), (3, "Hello", 42));
        assert_eq!(iter.next().unwrap().unwrap(), (3, "World", 42));
        assert_eq!(iter.next().unwrap().unwrap(), (4, "Hello", 42));
        assert_eq!(iter.next().unwrap().unwrap(), (4, "World", 42));
    }

    #[test]
    fn test_builder_combination_test_name() {
        let combination = BuilderCombination((5, false, "A text"));
        assert_eq!(combination.name(), Some("[5|false|A text]".into()));
        let combination = BuilderCombination((5, false, (Box::new(42), vec![5; 3])));
        assert_eq!(combination.name(), Some("[5|false|(42,[5,5,5])]".into()));
    }
}
