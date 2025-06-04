use rustest::{test, *};
use rustest_fixtures::Global;

use std::{
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
};
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
    let exec = env!("CARGO_BIN_EXE_scope_test");
    let mut command = std::process::Command::new(exec);
    command.env("NO_COLOR", "1");
    if let Some(options) = options {
        for opt in options {
            command.arg(opt);
        }
    };
    command.output()
}

struct Collector(HashMap<String, Vec<u32>>);

impl Collector {
    pub fn collect(stderr: &str) -> Self {
        let mut map = HashMap::new();
        for (key, value) in stderr
            .split('\n')
            .filter(|l| !l.is_empty())
            .map(|l| l.split_once(':').unwrap())
            .map(|(key, value)| (key.to_string(), value.parse().expect("Should parse a u32")))
        {
            map.entry(key).or_insert_with(Vec::new).push(value)
        }
        for vec in map.values_mut() {
            vec.sort();
            let mut prev = None;
            let mut new_vec = vec
                .iter()
                .filter_map(|current| match prev {
                    None => {
                        prev = Some((current, 1));
                        None
                    }
                    Some((prev_value, prev_count)) => {
                        if prev_value == current {
                            prev = Some((prev_value, prev_count + 1));
                            None
                        } else {
                            prev = Some((current, 1));
                            Some(prev_count)
                        }
                    }
                })
                .collect::<Vec<_>>();
            new_vec.push(prev.unwrap().1);
            *vec = new_vec;
        }
        Self(map)
    }

    pub fn check_build(&self, key: &str, nb: usize) {
        let key = format!("BUILD {} number", key);
        let vec = self.0.get(&key).unwrap();
        assert_eq!(vec.iter().sum::<u32>(), nb as u32);
        assert!(vec.iter().all(|v| *v == 1));
    }

    pub fn check_test(&self, key: &str, nb: usize, expected: &[u32]) {
        let key = format!("TEST {} number", key);
        let vec = self.0.get(&key).unwrap();
        assert_eq!(vec.iter().sum::<u32>(), nb as u32);
        assert_eq!(vec, expected);
    }
}

impl std::ops::Deref for Collector {
    type Target = HashMap<String, Vec<u32>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[test]
fn test_scope() {
    let output = run(Some(&["--test-threads=1"])).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let collector = Collector::collect(&stderr);
    // 4 for "scope" test, 1 for "make global" test, 2 extra (`Global<ScopeNumber>` is already cached) for "make global wrong"
    collector.check_build("scope", 7);
    collector.check_test("scope", 4, &[1, 1, 1, 1]);
    collector.check_build("matrix", 3);
    collector.check_test("matrix", 6, &[2, 2, 2]);
    collector.check_build("test", 2);
    collector.check_test("test", 6, &[4, 2]);
    collector.check_build("global", 1);
    collector.check_test("global", 6, &[6]);
    collector.check_test("make global", 6, &[6]);
    collector.check_test("make global wrong", 6, &[2, 2, 2]);
}

#[main]
fn main() {}
