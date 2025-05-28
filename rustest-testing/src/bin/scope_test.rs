use rustest::{test, *};
use rustest_fixtures::*;

#[test]
fn test_path(tmp_dir: Global<TempFile>) {
    eprintln!("TEST path is {}", tmp_dir.path().display());
}

#[test]
fn test_path_bis(tmp_dir: Global<TempFile>) {
    eprintln!("TEST path is {}", tmp_dir.path().display());
}

#[main]
fn main() {}
