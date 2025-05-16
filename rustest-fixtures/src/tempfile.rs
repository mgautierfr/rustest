use core::ops::Deref;

use rustest::FixtureScope;

/// A temporary directory.
///
/// A temporary file, generated with `tempfile` crate.
pub struct TempFile(pub tempfile::NamedTempFile);

impl TempFile {
    /// Convert the fixture to a [tempfile::NamedTempFile]
    pub fn into_inner(self) -> tempfile::NamedTempFile {
        self.0
    }
}

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

#[derive(Debug)]
pub struct TempFileBuilder;

impl rustest::Duplicate for TempFileBuilder {
    fn duplicate(&self) -> Self {
        Self
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
        vec![Self]
    }

    fn build(&self) -> rustest::FixtureCreationResult<Self::Fixt> {
        Ok(TempFile(
            tempfile::NamedTempFile::new_in(std::env::temp_dir())
                .map_err(|e| rustest::FixtureCreationError::new("TempFile", e))?,
        ))
    }
}
