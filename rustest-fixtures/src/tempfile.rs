use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

use rustest::{FixtureCreationResult, FixtureScope};

/// A temporary directory.
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
    type Builder = TempFileBuilder;
}

pub struct TempFileBuilder(Arc<OnceLock<FixtureCreationResult<Arc<tempfile::NamedTempFile>>>>);

impl rustest::Duplicate for TempFileBuilder {
    fn duplicate(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl rustest::TestName for TempFileBuilder {
    fn name(&self) -> Option<String> {
        None
    }
}

impl rustest::FixtureBuilder for TempFileBuilder {
    type Fixt = TempFile;
    type Type = tempfile::NamedTempFile;
    const SCOPE: FixtureScope = FixtureScope::Unique;

    fn setup(_ctx: &mut rustest::TestContext) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![Self(Arc::new(OnceLock::new()))]
    }

    fn build(&self) -> rustest::FixtureCreationResult<Self::Fixt> {
        self.0
            .get_or_init(|| {
                tempfile::NamedTempFile::new_in(std::env::temp_dir())
                    .map(|f| Arc::new(f))
                    .map_err(|e| rustest::FixtureCreationError::new("TempFile", e))
            })
            .as_ref()
            .map(|tmp| TempFile(Arc::clone(tmp)))
            .map_err(|e| e.clone())
    }
}
