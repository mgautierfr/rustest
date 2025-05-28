use rustest::{Duplicate, SubFixture};

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
pub struct Global<Source: SubFixture>(Source);

impl<Source: SubFixture> std::ops::Deref for Global<Source> {
    type Target = Source::Target;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<Source: SubFixture> rustest::Fixture for Global<Source> {
    type Type = Source::Type;
    type Proxy = GlobalProxy<Source>;
}

// Duplicated `Source::Proxy` already handle the inner cache on the value,
// So we don't need to have a Rc or else.
pub struct GlobalProxy<Source: SubFixture>(Source::Proxy);

impl<Source: SubFixture> rustest::Duplicate for GlobalProxy<Source> {
    fn duplicate(&self) -> Self {
        Self(self.0.duplicate())
    }
}

impl<Source: SubFixture> rustest::TestName for GlobalProxy<Source> {
    fn name(&self) -> Option<String> {
        self.0.name()
    }
}

impl<Source: SubFixture> rustest::FixtureProxy for GlobalProxy<Source> {
    type Fixt = Global<Source>;
    const SCOPE: rustest::FixtureScope = rustest::FixtureScope::Global;

    fn setup(ctx: &mut rustest::TestContext) -> Vec<Self>
    where
        Self: Sized,
    {
        if let Some(b) = ctx.get() {
            return b;
        }
        let proxies: Vec<_> = Source::Proxy::setup(ctx)
            .into_iter()
            .map(|b| Self(b))
            .collect();

        ctx.add::<Self>(proxies.duplicate());
        proxies
    }

    fn build(self) -> rustest::FixtureCreationResult<Self::Fixt> {
        Ok(Global(self.0.build()?))
    }
}
