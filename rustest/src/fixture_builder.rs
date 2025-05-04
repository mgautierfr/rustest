use core::{clone::Clone, marker::PhantomData};

use crate::{
    BuildableFixture, BuilderCall, BuilderCombination, CallArgs, Fixture, FixtureBuilder,
    FixtureCreationError, FixtureScope, LazyValue, SharedFixtureValue, TeardownFn, TestName,
};
use std::sync::{Arc, Mutex};

pub trait FixtureDef: std::fmt::Debug {
    type Fixt: BuildableFixture + std::fmt::Debug;
    type SubFixtures: std::fmt::Debug;
    type SubBuilders: std::fmt::Debug;
    const SCOPE: FixtureScope;
    fn setup_matrix(
        ctx: &mut crate::TestContext,
    ) -> Result<Vec<BuilderCombination<Self::SubBuilders>>, FixtureCreationError>;

    fn build_fixt(
        args: CallArgs<Self::SubFixtures>,
    ) -> Result<<Self::Fixt as Fixture>::Type, FixtureCreationError>;

    fn teardown() -> Option<Arc<TeardownFn<<Self::Fixt as Fixture>::Type>>>;
}

type InnerLazy<Def> = LazyValue<
    SharedFixtureValue<<<Def as FixtureDef>::Fixt as Fixture>::Type>,
    <Def as FixtureDef>::SubBuilders,
>;

pub struct Builder<Def: FixtureDef> {
    inner: Arc<Mutex<InnerLazy<Def>>>,
    name: Option<String>,
    _marker: PhantomData<Def>,
}

impl<Def: FixtureDef> Clone for Builder<Def> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            name: self.name.clone(),
            _marker: PhantomData,
        }
    }
}

impl<Def: FixtureDef> std::fmt::Debug for Builder<Def> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder").finish()
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

    fn setup(
        ctx: &mut crate::TestContext,
    ) -> std::result::Result<Vec<Self>, crate::FixtureCreationError> {
        if let Some(b) = ctx.get() {
            return Ok(b);
        }
        // We have to call this function for each combination of its fixtures.
        let builders = Def::setup_matrix(ctx)?;
        let inners = builders
            .into_iter()
            .map(|b| Self::new(b))
            .collect::<Vec<_>>();

        ctx.add::<Self>(inners.clone());
        Ok(inners)
    }

    fn build(&self) -> std::result::Result<Self::Fixt, crate::FixtureCreationError> {
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

    fn scope() -> crate::FixtureScope {
        Def::SCOPE
    }
}
