use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use super::{
    fixture::{
        Fixture, FixtureCreationResult, FixtureProxy, FixtureScope, FixtureTeardown, LazyValue,
        SharedFixtureValue, TeardownFn,
    },
    proxy_matrix::{CallArgs, Duplicate, MatrixSetup, ProxyCall, ProxyCombination, ProxyMatrix},
    test_name::TestName,
};

/// The definition of a fixture, use by Proxy to implement FixtureProxy.
#[doc(hidden)]
pub trait FixtureDef {
    type Fixt: Fixture;
    type SubFixtures;
    type SubProxies;
    const SCOPE: FixtureScope;

    fn build_fixt(
        args: CallArgs<Self::SubFixtures>,
    ) -> FixtureCreationResult<<Self::Fixt as Fixture>::Type>;

    fn teardown() -> Option<TeardownFn<<Self::Fixt as Fixture>::Type>>;
}

type InnerLazy<Def> =
    LazyValue<<<Def as FixtureDef>::Fixt as Fixture>::Type, <Def as FixtureDef>::SubProxies>;

#[doc(hidden)]
pub struct SharedProxy<Def: FixtureDef> {
    inner: Arc<Mutex<InnerLazy<Def>>>,
    name: Option<String>,
    _marker: PhantomData<Def>,
}

impl<Def: FixtureDef> Duplicate for SharedProxy<Def> {
    fn duplicate(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            name: self.name.clone(),
            _marker: PhantomData,
        }
    }
}

impl<Def: FixtureDef> TestName for SharedProxy<Def> {
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
}

impl<Def: FixtureDef> SharedProxy<Def>
where
    ProxyCombination<Def::SubProxies>: TestName,
{
    fn new(proxy: ProxyCombination<Def::SubProxies>) -> Self {
        let name = proxy.name();
        let inner = proxy.into();
        Self {
            inner: Arc::new(Mutex::new(inner)),
            name,
            _marker: PhantomData,
        }
    }
}

impl<Def: FixtureDef + 'static> FixtureProxy for SharedProxy<Def>
where
    ProxyCombination<Def::SubProxies>: TestName + ProxyCall<Def::SubFixtures>,
    ProxyMatrix<Def::SubProxies>: MatrixSetup<Def::SubProxies>,
    Def::Fixt: From<SharedFixtureValue<<Def::Fixt as Fixture>::Type>>,
{
    type Fixt = Def::Fixt;
    const SCOPE: FixtureScope = Def::SCOPE;

    fn setup(ctx: &mut crate::TestContext) -> Vec<Self> {
        if let Some(b) = ctx.get() {
            return b;
        }
        // We have to call this function for each combination of its fixtures.
        let proxies = ProxyMatrix::<Def::SubProxies>::setup(ctx);
        let inners = proxies
            .into_iter()
            .map(|b| Self::new(b))
            .collect::<Vec<_>>();

        ctx.add::<Self>(inners.duplicate());
        inners
    }

    fn build(self) -> FixtureCreationResult<Self::Fixt> {
        let inner = self
            .inner
            .lock()
            .unwrap()
            .get(|args| Ok((Def::build_fixt(args)?, Def::teardown())))?;
        Ok(inner.into())
    }
}

#[doc(hidden)]
pub struct OnceProxy<Def: FixtureDef> {
    sub_proxies: ProxyCombination<Def::SubProxies>,
    name: Option<String>,
    _marker: PhantomData<Def>,
}

impl<Def: FixtureDef> Duplicate for OnceProxy<Def>
where
    ProxyCombination<Def::SubProxies>: Duplicate,
{
    fn duplicate(&self) -> Self {
        Self {
            sub_proxies: self.sub_proxies.duplicate(),
            name: self.name.clone(),
            _marker: PhantomData,
        }
    }
}

impl<Def: FixtureDef> TestName for OnceProxy<Def> {
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
}

impl<Def: FixtureDef> OnceProxy<Def>
where
    ProxyCombination<Def::SubProxies>: TestName,
{
    fn new(sub_proxies: ProxyCombination<Def::SubProxies>) -> Self {
        let name = sub_proxies.name();
        Self {
            sub_proxies,
            name,
            _marker: PhantomData,
        }
    }
}

impl<Def: FixtureDef + 'static> FixtureProxy for OnceProxy<Def>
where
    ProxyCombination<Def::SubProxies>: TestName + ProxyCall<Def::SubFixtures> + Duplicate,
    ProxyMatrix<Def::SubProxies>: MatrixSetup<Def::SubProxies>,
    Def::Fixt: From<FixtureTeardown<<Def::Fixt as Fixture>::Type>>,
{
    type Fixt = Def::Fixt;
    const SCOPE: FixtureScope = Def::SCOPE;

    fn setup(ctx: &mut crate::TestContext) -> Vec<Self> {
        // We have to call this function for each combination of its fixtures.
        let proxies = ProxyMatrix::<Def::SubProxies>::setup(ctx);
        proxies.into_iter().map(|b| Self::new(b)).collect()
    }

    fn build(self) -> FixtureCreationResult<Self::Fixt> {
        let value = self.sub_proxies.call(Def::build_fixt)?;
        Ok(FixtureTeardown::new(value, Def::teardown()).into())
    }
}
