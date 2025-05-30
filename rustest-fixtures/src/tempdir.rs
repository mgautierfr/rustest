use rustest::{
    Duplicate, Fixture, FixtureCreationError, FixtureCreationResult, FixtureProxy, FixtureScope,
    TestContext, TestName,
};

/// A temporary directory.
///
/// A temporary directory, generated with `tempfile` crate.
pub struct TempDir(tempfile::TempDir);

impl std::ops::Deref for TempDir {
    type Target = tempfile::TempDir;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Fixture for TempDir {
    type Type = tempfile::TempDir;
    type Proxy = Proxy;
}

pub struct Proxy;

impl Duplicate for Proxy {
    fn duplicate(&self) -> Self {
        Self
    }
}

impl TestName for Proxy {
    fn name(&self) -> Option<String> {
        None
    }
}

impl FixtureProxy for Proxy {
    type Fixt = TempDir;
    const SCOPE: FixtureScope = FixtureScope::Once;

    fn setup(_ctx: &mut TestContext) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![Self]
    }

    fn build(self) -> FixtureCreationResult<Self::Fixt> {
        tempfile::tempdir_in(std::env::temp_dir())
            .map(TempDir)
            .map_err(|e| FixtureCreationError::new("TempDir", e))
    }
}
