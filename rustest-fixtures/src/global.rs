use rustest::*;
use std::sync::{Arc, Mutex};
/// Transform a fixture into a global fixture.
///
/// ```rust
/// # use rustest::{test, *};
/// # use rustest_fixtures::Global;
/// #[fixture]
/// fn MyFixture() -> u32 { 5 }
///
/// #[test]
/// fn my_test(n: Global<MyFixture>) { assert_eq!(*n, 5); }
///
/// # #[main]
/// # fn main() {}
/// ```
/// is equivalent to
/// ```rust
/// # use rustest::{test, *};
/// #[fixture(scope=global)]
/// fn MyFixture() -> u32 { 5 }
///
/// #[test]
/// fn my_test(n: MyFixture) { assert_eq!(*n, 5); }
///
/// # #[main]
/// # fn main() {}
/// ```
///
/// But with `Global`, you define the fixture to be global at test level.
/// It can be useful when composing external fixtures which can be define in external crate.
///

#[derive(Clone)]
pub struct Global<Source>(::rustest::SharedFixtureValue<Source>)
where
    Source: SubFixture;

impl<Source> Fixture for Global<Source>
where
    Source: SubFixture,
{
    type Type = Source::Type;
    type Proxy = Proxy<Source>;
}

impl<Source> ::std::ops::Deref for Global<Source>
where
    Source: SubFixture,
{
    type Target = <Self as ::rustest::Fixture>::Type;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Proxy<Source: SubFixture> {
    inner: Arc<Mutex<LazyValue<Source, (Source::Proxy,)>>>,
    name: Option<String>,
}

impl<Source: SubFixture> Duplicate for Proxy<Source> {
    fn duplicate(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            name: self.name.clone(),
        }
    }
}

impl<Source: SubFixture> TestName for Proxy<Source> {
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
}

impl<Source: SubFixture> Proxy<Source>
where
    ProxyCombination<(Source::Proxy,)>: TestName,
{
    fn new(proxy: ProxyCombination<(Source::Proxy,)>) -> Self {
        let name = proxy.name();
        let inner = proxy.into();
        Self {
            inner: Arc::new(Mutex::new(inner)),
            name,
        }
    }
}

impl<Source: SubFixture> FixtureProxy for Proxy<Source> {
    type Fixt = Global<Source>;
    const SCOPE: FixtureScope = FixtureScope::Global;

    fn setup(ctx: &mut TestContext) -> Vec<Self> {
        if let Some(b) = ctx.get() {
            return b;
        }
        // We have to call this function for each combination of its fixtures.
        let proxies = ProxyMatrix::<(Source::Proxy,)>::setup(ctx);
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
            .get(|CallArgs((source,))| Ok((source, None)))?;
        Ok(Global(inner))
    }
}
