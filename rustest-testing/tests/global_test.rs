use rustest::{test, *};
use rustest_fixtures::Global;

use std::sync::atomic::{AtomicU32, Ordering};
static INC_NUMBER: AtomicU32 = AtomicU32::new(0);

#[fixture]
fn IncNumber() -> u32 {
    println!("Build number");
    INC_NUMBER.fetch_add(1, Ordering::Relaxed)
}

#[test]
fn test_global_0(number: Global<IncNumber>) {
    assert_eq!(*number, 0);
}

#[test]
fn test_global_1(number: Global<IncNumber>) {
    assert_eq!(*number, 0);
}

fn run(options: Option<&[&str]>) -> std::io::Result<std::process::Output> {
    let exec = env!("CARGO_BIN_EXE_global_test");
    let mut command = std::process::Command::new(&exec);
    command.env("NO_COLOR", "1");
    options.map(|options| {
        for opt in options {
            command.arg(opt);
        }
    });
    command.output()
}

#[test]
fn test_global_tempdir() {
    let output = run(None).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let outlines = stderr
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();
    println!("{:?}", outlines);
    assert_eq!(outlines.len(), 2);
    assert_eq!(outlines[0], outlines[1]);
}

#[main]
fn main() {}
