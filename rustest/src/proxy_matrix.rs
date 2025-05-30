use std::{cmp::PartialEq, sync::Arc};

use super::{
    fixture::{FixtureCreationResult, FixtureProxy},
    test::TestContext,
    test_name::TestName,
};

#[doc(hidden)]
pub struct CallArgs<Types>(pub Types);

#[doc(hidden)]
pub struct ProxyCombination<KnownType>(KnownType);

impl<KnownType> PartialEq<KnownType> for ProxyCombination<KnownType>
where
    KnownType: PartialEq,
{
    fn eq(&self, other: &KnownType) -> bool {
        self.0.eq(other)
    }
}

pub trait ProxyCall<Args> {
    fn call<F, Output>(self, f: F) -> FixtureCreationResult<Output>
    where
        F: FnOnce(CallArgs<Args>) -> FixtureCreationResult<Output>;
}

macro_rules! impl_fixture_combination_call {
    ((), ()) => {
        impl ProxyCall<()> for ProxyCombination<()> where
        {
            fn call<F, Output>(
                self,
                f: F,
            ) -> FixtureCreationResult<Output>
                where
                F: FnOnce(CallArgs<()>) -> FixtureCreationResult<Output>,
            {
                f(CallArgs(()))
            }
        }
    };
    (($($types:tt),+), ($($names:ident),+)) => {

        impl<$($types),+> ProxyCall<($($types::Fixt),+,)> for ProxyCombination<($($types),+,)> where
            $($types : FixtureProxy + 'static),+ ,
        {
            fn call<F, Output>(
                self,
                f: F,
            ) -> FixtureCreationResult<Output>
                where
                F: FnOnce(CallArgs<($($types::Fixt),+,)>) -> FixtureCreationResult<Output>,
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
/// `ProxyMatrix` is used to manage a collection of fixtures.
/// It acts as an increasing matrix of dimension N as we feed it with new fixtures vector.
#[doc(hidden)]
#[derive(Default)]
pub struct ProxyMatrix<ProxiesTypes> {
    proxies: ProxiesTypes,
    multiple: bool,
}

pub trait MatrixSetup<SubProxies> {
    fn setup(_ctx: &mut TestContext) -> Vec<ProxyCombination<SubProxies>>;
}

impl<T> ProxyMatrix<T> {
    /// Does the ProxyMatrix as multiple combination ?
    pub fn is_multiple(&self) -> bool {
        self.multiple
    }
}

impl ProxyMatrix<()> {
    /// Creates a new `ProxyMatrix` with 0 dimension.
    pub fn new() -> Self {
        Self {
            proxies: (),
            multiple: false,
        }
    }
}

impl ProxyMatrix<()> {
    /// Feeds new fixtures into the matrix.
    ///
    /// # Arguments
    ///
    /// * `new_fixs` - A vector of new fixtures to feed into the matrix.
    ///
    /// # Returns
    ///
    /// A new `ProxyMatrix` of dimension 1 containing the fed fixtures.
    pub fn feed<T>(self, new_fixs: Vec<T>) -> ProxyMatrix<(Vec<T>,)> {
        let multiple = self.multiple || new_fixs.len() > 1;
        ProxyMatrix {
            proxies: (new_fixs,),
            multiple,
        }
    }

    /// Call the function f... with no fixture as this ProxyMatrix is dimension 0.
    pub fn flatten(self) -> Vec<ProxyCombination<()>> {
        vec![ProxyCombination(())]
    }
}

impl MatrixSetup<()> for ProxyMatrix<()> {
    fn setup(_ctx: &mut TestContext) -> Vec<ProxyCombination<()>> {
        vec![ProxyCombination(())]
    }
}

macro_rules! impl_fixture_test_name {
    ((), ()) => {
        impl TestName for ProxyCombination<()>
        {
            fn name(&self) -> Option<String> {
                None
            }
        }
    };
    (($($types:tt),+), ($($names:ident),+)) => {
        impl< $($types),+ > TestName for ProxyCombination<($($types),+,)>
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
/// Duplicated proxies must produce the **SAME** fitxure. `Clone` trait do not provide
/// this sementics.
/// Producing the same value implies proxies should have some kind of shared cache
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

impl<T: Duplicate> Duplicate for Arc<T> {
    fn duplicate(&self) -> Self {
        Arc::clone(self)
    }
}

macro_rules! iter_proxy {
    (@call $collect:expr ; $last_name:ident ; $last_proxy:expr ; ) => {
        for $last_name in $last_proxy.iter() {
            let combination = ProxyCombination(($last_name.duplicate(), ));
            $collect.push(combination)
        }
    };
    (@call $collect:expr ; $last_name:ident ; $last_proxy:expr ; $($known:tt),*) => {
        for $last_name in $last_proxy.iter() {
            let combination = ProxyCombination(($($known.duplicate()),*, $last_name.duplicate()));
            $collect.push(combination)
        }
    };
    (@call $collect:expr ; $first_name:tt, $($other_names:ident),* ; $first_proxy:expr, $($other_proxies:expr),* ; ) => {
        for $first_name in $first_proxy.iter() {
            iter_proxy!(@call $collect ; $($other_names),* ; $($other_proxies),* ; $first_name)
        }
    };
    (@call $collect:expr ; $first_name:tt, $($other_names:ident),* ; $first_proxy:expr, $($other_proxies:expr),* ; $($known:expr),*) => {
        for $first_name in $first_proxy.iter() {
            iter_proxy!(@call $collect ; $($other_names),* ; $($other_proxies),* ; $($known),* , $first_name)
        }
    };
}

macro_rules! impl_fixture_call {
    (@proxy_setup, $proxy_matrix:expr, $ctx:expr, $proxy:ident) => {{
        let proxy_matrix = $proxy_matrix.feed($proxy::setup($ctx));
        proxy_matrix.flatten()
    }};
    (@proxy_setup, $proxy_matrix:expr, $ctx:expr, $proxy:ident, $($types:tt),+) => {{
        let proxy_matrix = $proxy_matrix.feed($proxy::setup($ctx));
        impl_fixture_call!(@proxy_setup, proxy_matrix, $ctx, $($types),+)
    }};

    (($($types:tt),+), ($($bnames:ident),+), ($($fnames:ident),+)) => {

        impl<$($types),+> ProxyMatrix<($(Vec<$types>),+,)> where
            $($types : Duplicate + TestName + 'static),+ ,
        {
            pub fn flatten(self) -> Vec<ProxyCombination<($($types),+,)>>
            {
                let ($($bnames),+, ) = self.proxies;
                let mut output = vec![];
                iter_proxy!(@call output ; $($fnames),+ ; $($bnames),+ ;);
                output
            }
        }


        impl<$($types),+> MatrixSetup<($($types),+,)> for ProxyMatrix<($($types),+,)> where
            $($types : Duplicate + FixtureProxy + TestName + 'static),+ ,
        {
            fn setup(ctx: &mut TestContext) -> Vec<ProxyCombination<($($types),+,)>> {
                let proxy_matrix = ProxyMatrix::new();
                impl_fixture_call!(@proxy_setup, proxy_matrix, ctx, $($types),+)
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
        impl<$($types),+> ProxyMatrix<($(Vec<$types>),+,)> where
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
            /// A new `ProxyMatrix` containing the fed fixtures.
            pub fn feed<T>(self, new_fixs: Vec<T>) -> ProxyMatrix<($(Vec<$types>),+ ,Vec<T>)> {
                let multiple = self.multiple || new_fixs.len() > 1;
                                let ($($names),+, ) = self.proxies;
                                let proxies = ($($names),+ , new_fixs);
                ProxyMatrix { proxies, multiple }
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
    use super::*;
    use crate::{Fixture, FixtureRegistry, FixtureScope, TestContext};

    struct DummyFixture<T>(T);

    #[derive(Debug, PartialEq)]
    struct DummyFixtureProxy<T>(T);

    impl<T: Copy> Duplicate for DummyFixtureProxy<T> {
        fn duplicate(&self) -> Self {
            Self(self.0)
        }
    }
    impl<T> FixtureProxy for DummyFixtureProxy<T>
    where
        T: Send + Copy + std::fmt::Display + 'static,
    {
        type Fixt = DummyFixture<T>;
        const SCOPE: FixtureScope = FixtureScope::Unique;

        fn setup(_ctx: &mut TestContext) -> Vec<Self>
        where
            Self: Sized,
        {
            unimplemented!()
        }

        fn build(self) -> FixtureCreationResult<DummyFixture<T>> {
            Ok(DummyFixture(self.0))
        }
    }

    impl<T> Fixture for DummyFixture<T>
    where
        T: Send + Copy + std::fmt::Display + 'static,
    {
        type Type = T;
        type Proxy = DummyFixtureProxy<T>;
    }
    impl<T> std::ops::Deref for DummyFixture<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<T: std::fmt::Display> TestName for DummyFixtureProxy<T> {
        fn name(&self) -> Option<String> {
            None
        }
    }

    #[test]
    fn test_empty_fixture_registry() {
        let mut registry = FixtureRegistry::new();
        assert!(registry.get::<DummyFixtureProxy<i32>>().is_none());
    }

    #[test]
    fn test_fixture_registry() {
        let mut registry = FixtureRegistry::new();
        registry.add(vec![DummyFixtureProxy(1u32), DummyFixtureProxy(2u32)]);
        let proxies = registry.get::<DummyFixtureProxy<u32>>().unwrap();
        assert_eq!(proxies.len(), 2);
        assert_eq!(proxies[0], DummyFixtureProxy(1));
        assert_eq!(proxies[1], DummyFixtureProxy(2));
        assert!(registry.get::<DummyFixtureProxy<u16>>().is_none());
    }

    #[test]
    fn test_proxy_matrix() {
        let matrix = ProxyMatrix::new()
            .feed(vec![
                DummyFixtureProxy(1),
                DummyFixtureProxy(2),
                DummyFixtureProxy(3),
            ])
            .feed(vec![DummyFixtureProxy("Hello"), DummyFixtureProxy("World")]);
        assert_eq!(
            matrix.proxies.0,
            vec![
                DummyFixtureProxy(1),
                DummyFixtureProxy(2),
                DummyFixtureProxy(3)
            ]
        );
        assert_eq!(
            matrix.proxies.1,
            vec![DummyFixtureProxy("Hello"), DummyFixtureProxy("World")]
        );
    }

    #[test]
    fn test_matrix_caller() {
        let matrix = ProxyMatrix::new().feed(vec![
            DummyFixtureProxy(1),
            DummyFixtureProxy(2),
            DummyFixtureProxy(3),
        ]);
        let matrix = matrix.feed(vec![DummyFixtureProxy("Hello"), DummyFixtureProxy("World")]);
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
        let matrix = ProxyMatrix::new().feed(vec![
            DummyFixtureProxy(1),
            DummyFixtureProxy(2),
            DummyFixtureProxy(3),
        ]);
        let matrix = matrix.feed(vec![DummyFixtureProxy("Hello"), DummyFixtureProxy("World")]);
        let matrix = matrix.feed(vec![DummyFixtureProxy(42)]);
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
        struct P<T>(T);

        impl<T> TestName for P<T>
        where
            T: crate::ParamName,
        {
            fn name(&self) -> Option<String> {
                Some(self.0.param_name())
            }
        }
        let combination = ProxyCombination((P(5), P(false), P("A text")));
        assert_eq!(combination.name(), Some("[5|false|A text]".into()));
        let combination = ProxyCombination((P(5), P(false), P((Box::new(42), vec![5; 3]))));
        assert_eq!(combination.name(), Some("[5|false|(42,[5,5,5])]".into()));
    }
}
