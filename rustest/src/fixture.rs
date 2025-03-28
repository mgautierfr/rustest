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

pub fn get_fixture<Fix>(
    ctx: &mut TestContext,
) -> std::result::Result<Vec<Fix>, FixtureCreationError>
where
    Fix: Fixture + Any,
{
    ctx.get_fixture()
}

#[derive(Default)]
pub struct FixtureRegistry {
    pub fixtures: std::collections::HashMap<TypeId, Box<dyn Any>>,
}

impl FixtureRegistry {
    pub fn new() -> Self {
        Self {
            fixtures: Default::default(),
        }
    }

    pub fn add<F>(&mut self, value: Vec<F::InnerType>)
    where
        F: Fixture + 'static,
        F::InnerType: Clone + 'static,
    {
        self.fixtures
            .insert(std::any::TypeId::of::<F>(), Box::new(value));
    }

    pub fn get<F>(&mut self) -> Option<Vec<F::InnerType>>
    where
        F: Fixture + 'static,
        F::InnerType: Clone + 'static,
    {
        self.fixtures.get(&std::any::TypeId::of::<F>()).map(|a| {
            let fixture = a.downcast_ref::<Vec<F::InnerType>>().unwrap();
            fixture.clone()
        })
    }
}

pub type TeardownFn<T> = dyn Fn(&mut T) + Send + RefUnwindSafe + UnwindSafe + Sync;

#[derive(Clone)]
pub struct FixtureTeardown<T> {
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

impl<T> std::ops::DerefMut for FixtureTeardown<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Drop for FixtureTeardown<T> {
    fn drop(&mut self) {
        let teardown = self.teardown.take();
        if let Some(t) = teardown {
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
}

impl FixtureMatrix<()> {
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
        }
    }
}

impl<KnownTypes> From<Vec<KnownTypes>> for FixtureMatrix<KnownTypes> {
    fn from(fixtures: Vec<KnownTypes>) -> Self {
        Self { fixtures }
    }
}
impl FixtureMatrix<()> {
    pub fn feed<T: Clone>(self, new_fixs: Vec<T>) -> FixtureMatrix<((), T)> {
        let fixtures = new_fixs
            .iter()
            .map(move |new| ((), new.clone()))
            .collect::<Vec<_>>();
        fixtures.into()
    }
}

impl<KnownTypes> FixtureMatrix<((), KnownTypes)>
where
    KnownTypes: Clone,
{
    pub fn feed<T: Clone>(self, new_fixs: Vec<T>) -> FixtureMatrix<(((), KnownTypes), T)> {
        let fixtures = self
            .fixtures
            .into_iter()
            .flat_map(|existing| {
                new_fixs
                    .iter()
                    .map(move |new| (existing.clone(), new.clone()))
            })
            .collect::<Vec<_>>();
        fixtures.into()
    }
}

pub trait FixtureName {
    fn name(&self) -> String;
}

macro_rules! impl_fixt_name {
    (($($types:tt),+), ($($names:ident),+)) => {
        impl< $($types),+ > FixtureName for ($($types),+,)
            where
                $($types : Send + UnwindSafe + FixtureName + 'static),+ , {
            fn name(&self) -> String {
                impl_fixt_name!(@expand, self, $($names),+);
                impl_fixt_name!(@to_name, $($names),+);
                let vec = vec![$($names),+];
                vec.join("|")
            }
        }
    };
    (@expand, $tup:ident, $($fixs:ident),+) => {
        let ($($fixs),+ ,) = $tup;
    };
    (@to_name, $name:tt) => {
        let $name = $name.name();
    };
    (@to_name, $name:tt, $($names:tt),+) => {
        let $name = $name.name();
        impl_fixt_name!(@to_name, $($names),+)
    };
}

impl_fixt_name!((F0), (f0));
impl_fixt_name!((F0, F1), (f0, f1));
impl_fixt_name!((F0, F1, F2), (f0, f1, f2));
impl_fixt_name!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_fixt_name!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_fixt_name!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_fixt_name!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_fixt_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_fixt_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_fixt_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_fixt_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_fixt_name!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

pub struct MatrixCaller<T> {
    fixtures: Vec<T>,
}

impl MatrixCaller<()> {
    pub fn call<F, Output>(
        self,
        f: F,
    ) -> Vec<(String, Box<dyn FnOnce() -> Output + Send + UnwindSafe>)>
    where
        F: Fn() -> Output + Send + Sync + UnwindSafe + RefUnwindSafe + 'static,
    {
        vec![("".into(), Box::new(f))]
    }
}

macro_rules! impl_call {
    (($($types:tt),+), ($($names:ident),+)) => {
        impl< $($types),+ > MatrixCaller< ( $($types),+ ,) > where
                $($types : Send + UnwindSafe + FixtureName + 'static),+ ,

        {
            pub fn call<F, Output>(
                self,
                f: F,
            ) -> Vec<(String, Box<dyn FnOnce() -> Output + Send + UnwindSafe>)>
                where
                F: Fn($($types),+) -> Output + Send + Sync + UnwindSafe + RefUnwindSafe + 'static,
            {
                let caller = Arc::new(f);
                self.fixtures
                    .into_iter()
                    .map(|fix| {
                        let name = fix.name();
                        impl_call!(@expand, fix, $($names),+);
                        let caller = Arc::clone(&caller);
                        let runner = move || caller($($names),+);
                        (name, Box::new(runner) as Box<dyn FnOnce() -> Output + Send + UnwindSafe>)
                    })
                    .collect()
            }
        }
    };


    (@expand, $tup:ident, $($fixs:ident),+) => {
        let ($($fixs),+ ,) = $tup;
    };
}

impl_call!((F0), (f0));
impl_call!((F0, F1), (f0, f1));
impl_call!((F0, F1, F2), (f0, f1, f2));
impl_call!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_call!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_call!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_call!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_call!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

impl From<FixtureMatrix<()>> for MatrixCaller<()> {
    fn from(m: FixtureMatrix<()>) -> Self {
        Self {
            fixtures: m.fixtures,
        }
    }
}

macro_rules! impl_fixture_from {
    (($($types:tt),+), ($($names:ident),+)) => {
        impl<$($types),+> From<FixtureMatrix<impl_fixture_from!(@iter, $($types),+)>> for MatrixCaller<($($types),+,)> {
            fn from(m: FixtureMatrix<impl_fixture_from!(@iter, $($types),+)>) -> Self {
                let fixtures = m
                    .fixtures
                    .into_iter()
                    .map(|v| {
                        impl_fixture_from!(@expand, v, $($names),+);
                        ($($names),+,)
                    })
                    .collect();
                Self { fixtures }
            }
        }
    };

    (@iter, $first:tt) => { ((), $first) };
    (@iter, $first:tt, $second:tt) => { (impl_fixture_from!(@iter, $first), $second) };
    (@iter, $first:tt, $second:tt, $($other:tt),* ) => { (impl_fixture_from!(@iter, $first, $second), $($other),*) };


    (@expand, $tup:ident, $($fixs:ident),+) => {
        let impl_fixture_from!(@iter, $($fixs),+) = $tup;
    };
}

impl_fixture_from!((F0), (f0));
impl_fixture_from!((F0, F1), (f0, f1));
impl_fixture_from!((F0, F1, F2), (f0, f1, f2));
impl_fixture_from!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_fixture_from!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_fixture_from!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_fixture_from!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_fixture_from!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_fixture_from!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_fixture_from!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_fixture_from!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_fixture_from!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

#[derive(Clone)]
pub struct FixtureParam<T>(pub T);

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

impl<T> FixtureParam<T> {
    pub fn into(self) -> T {
        self.0
    }
}
