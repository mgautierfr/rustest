use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use super::{
    fixture::{
        Fixture, FixtureBuilder, FixtureCreationResult, FixtureScope, LazyValue,
        SharedFixtureValue, TeardownFn,
    },
    fixture_matrix::{BuilderCall, BuilderCombination, CallArgs, Duplicate},
    test_name::TestName,
};

pub trait BuildableFixture: Fixture {
    fn new(v: SharedFixtureValue<Self::Type>) -> Self;
}

/// The definition of a fixture, use by Builder to implement FixtureBuilder.
#[doc(hidden)]
pub trait FixtureDef {
    type Fixt: BuildableFixture;
    type SubFixtures;
    type SubBuilders;
    const SCOPE: FixtureScope;
    fn setup_matrix(ctx: &mut crate::TestContext) -> Vec<BuilderCombination<Self::SubBuilders>>;

    fn build_fixt(
        args: CallArgs<Self::SubFixtures>,
    ) -> FixtureCreationResult<<Self::Fixt as Fixture>::Type>;

    fn teardown() -> Option<Arc<TeardownFn<<Self::Fixt as Fixture>::Type>>>;
}

type InnerLazy<Def> = LazyValue<
    SharedFixtureValue<<<Def as FixtureDef>::Fixt as Fixture>::Type>,
    <Def as FixtureDef>::SubBuilders,
>;

#[doc(hidden)]
pub struct Builder<Def: FixtureDef> {
    inner: Arc<Mutex<InnerLazy<Def>>>,
    name: Option<String>,
    _marker: PhantomData<Def>,
}

impl<Def: FixtureDef> Duplicate for Builder<Def> {
    fn duplicate(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            name: self.name.clone(),
            _marker: PhantomData,
        }
    }
}

impl<Def: FixtureDef> TestName for Builder<Def> {
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
}

impl<Def: FixtureDef> Builder<Def>
where
    BuilderCombination<Def::SubBuilders>: TestName,
{
    fn new(builder: BuilderCombination<Def::SubBuilders>) -> Self {
        let name = builder.name();
        let inner = builder.into();
        Self {
            inner: Arc::new(Mutex::new(inner)),
            name,
            _marker: PhantomData,
        }
    }
}

impl<Def: FixtureDef + 'static> FixtureBuilder for Builder<Def>
where
    BuilderCombination<Def::SubBuilders>: TestName + BuilderCall<Def::SubFixtures>,
{
    type Fixt = Def::Fixt;
    type Type = <Def::Fixt as Fixture>::Type;
    const SCOPE: FixtureScope = Def::SCOPE;

    fn setup(ctx: &mut crate::TestContext) -> Vec<Self> {
        if let Some(b) = ctx.get() {
            return b;
        }
        // We have to call this function for each combination of its fixtures.
        let builders = Def::setup_matrix(ctx);
        let inners = builders
            .into_iter()
            .map(|b| Self::new(b))
            .collect::<Vec<_>>();

        ctx.add::<Self>(inners.duplicate());
        inners
    }

    fn build(&self) -> FixtureCreationResult<Self::Fixt> {
        let inner = self
            .inner
            .lock()
            .unwrap()
            .get(|args| {
                Ok(SharedFixtureValue::new(
                    Def::build_fixt(args)?,
                    Def::teardown(),
                ))
            })?
            .clone();
        Ok(Self::Fixt::new(inner))
    }
}
