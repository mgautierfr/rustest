use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

use rustest::{FixtureCreationResult, FixtureScope};

/// A temporary file.
///
/// A temporary file, generated with `tempfile` crate.
#[derive(Clone)]
pub struct TempFile(Arc<tempfile::NamedTempFile>);

impl Deref for TempFile {
    type Target = tempfile::NamedTempFile;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl rustest::Fixture for TempFile {
    type Type = tempfile::NamedTempFile;
    type Proxy = TempFileProxy;
}

pub struct TempFileProxy(Arc<OnceLock<FixtureCreationResult<Arc<tempfile::NamedTempFile>>>>);

impl rustest::Duplicate for TempFileProxy {
    fn duplicate(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl rustest::TestName for TempFileProxy {
    fn name(&self) -> Option<String> {
        None
    }
}

impl rustest::FixtureProxy for TempFileProxy {
    type Fixt = TempFile;
    const SCOPE: FixtureScope = FixtureScope::Unique;

    fn setup(_ctx: &mut rustest::TestContext) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![Self(Arc::new(OnceLock::new()))]
    }

    fn build(self) -> rustest::FixtureCreationResult<Self::Fixt> {
        self.0
            .get_or_init(|| {
                tempfile::NamedTempFile::new_in(std::env::temp_dir())
                    .map(Arc::new)
                    .map_err(|e| rustest::FixtureCreationError::new("TempFile", e))
            })
            .as_ref()
            .map(|tmp| TempFile(Arc::clone(tmp)))
            .map_err(|e| e.clone())
    }
}
