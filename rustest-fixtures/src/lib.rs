//! This crate is a set of standard fixtures for rustest.
//!
//! This crate is pretty young and a the number of fixture is small.
//! If you have a need for new standard fixture, issue or PR are welcomed.

mod global;
mod tempdir;
mod tempfile;

pub use global::Global;
pub use tempdir::TempDir;
pub use tempfile::TempFile;
