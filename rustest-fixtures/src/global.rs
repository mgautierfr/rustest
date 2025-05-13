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
    type Builder = GlobalBuilder<Source>;
}

#[derive(Debug)]
// Duplicated `Source::Builder` already handle the inner cache on the value,
// So we don't need to have a Rc or else.
pub struct GlobalBuilder<Source: SubFixture>(Source::Builder);

impl<Source: SubFixture> rustest::Duplicate for GlobalBuilder<Source> {
    fn duplicate(&self) -> Self {
        Self(self.0.duplicate())
    }
}

impl<Source: SubFixture> rustest::TestName for GlobalBuilder<Source> {
    fn name(&self) -> Option<String> {
        self.0.name()
    }
}

impl<Source: SubFixture> rustest::FixtureBuilder for GlobalBuilder<Source> {
    type Fixt = Global<Source>;
    type Type = Source::Type;
    const SCOPE: rustest::FixtureScope = rustest::FixtureScope::Global;

    fn setup(ctx: &mut rustest::TestContext) -> Vec<Self>
    where
        Self: Sized,
    {
        if let Some(b) = ctx.get() {
            return b;
        }
        let builders: Vec<_> = Source::Builder::setup(ctx)
            .into_iter()
            .map(|b| Self(b))
            .collect();

        ctx.add::<Self>(builders.duplicate());
        builders
    }

    fn build(&self) -> std::result::Result<Self::Fixt, rustest::FixtureCreationError>
    where
        Self: Sized,
    {
        Ok(Global(self.0.build()?))
    }
}
