use rustest::{FixtureProxy, FixtureScope};

/// A temporary file.
///
/// A temporary file, generated with `tempfile` crate.
pub struct TempFile(tempfile::NamedTempFile);

impl std::ops::Deref for TempFile {
    type Target = tempfile::NamedTempFile;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl rustest::Fixture for TempFile {
    type Type = tempfile::NamedTempFile;
    type Proxy = Proxy;
}

pub struct Proxy;

impl rustest::Duplicate for Proxy {
    fn duplicate(&self) -> Self {
        Self
    }
}

impl rustest::TestName for Proxy {
    fn name(&self) -> Option<String> {
        None
    }
}

impl FixtureProxy for Proxy {
    type Fixt = TempFile;
    const SCOPE: FixtureScope = FixtureScope::Once;

    fn setup(_ctx: &mut rustest::TestContext) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![Self]
    }

    fn build(self) -> rustest::FixtureCreationResult<Self::Fixt> {
        tempfile::NamedTempFile::new_in(std::env::temp_dir())
            .map(TempFile)
            .map_err(|e| rustest::FixtureCreationError::new("TempFile", e))
    }
}
