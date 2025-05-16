use regex::Regex;
use std::collections::HashMap;
use std::process::Output;
use std::sync::LazyLock;

fn run(options: Option<&[&str]>) -> std::io::Result<std::process::Output> {
    let exec = env!("CARGO_BIN_EXE_ignored_test");
    let mut command = std::process::Command::new(&exec);
    command.env("NO_COLOR", "1");
    options.map(|options| {
        for opt in options {
            command.arg(opt);
        }
    });
    command.output()
}

#[derive(Default, Debug, PartialEq)]
struct TestResult {
    pub tested: usize,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub measured: usize,
    pub filtered_out: usize,
}

struct TestCollector {
    output: Output,
    counter: HashMap<Vec<u8>, usize>,
    result: TestResult,
}

impl TestCollector {
    fn collect(output: Output) -> Self {
        let mut result: TestResult = Default::default();
        let mut counter = HashMap::new();
        let mut parse_line = |line: &[u8]| {
            if line.starts_with(b"running ") {
                let l = String::from_utf8_lossy(line);
                println!("{}", l);
                static RE: LazyLock<Regex> = LazyLock::new(|| {
                    Regex::new(r"running (?<tested>[[:digit:]]+) tests?").unwrap()
                });
                let caps = RE.captures(&l).unwrap();
                let tested = caps["tested"].parse().unwrap();
                result = TestResult { tested, ..result }
            } else if line.starts_with(b"test result:") {
                let l = String::from_utf8_lossy(line);
                static RE: LazyLock<Regex> = LazyLock::new(|| {
                    let sub_regex = ["passed", "failed", "ignored", "measured", "filtered out"]
                        .into_iter()
                        .map(|v| format!("(?<{}>[[:digit:]]+) {v};", v.replace(" ", "_")))
                        .collect::<Vec<_>>()
                        .join(" ");
                    Regex::new(&format!("test result: (ok|failed). {sub_regex} finished in [[:digit:]]+.[[:digit:]]+s")).unwrap()
                });
                let caps = RE.captures(&l).unwrap();
                let passed = caps["passed"].parse().unwrap();
                let failed = caps["failed"].parse().unwrap();
                let ignored = caps["ignored"].parse().unwrap();
                let measured = caps["measured"].parse().unwrap();
                let filtered_out = caps["filtered_out"].parse().unwrap();
                result = TestResult {
                    passed,
                    failed,
                    ignored,
                    measured,
                    filtered_out,
                    ..result
                };
            } else if !line.is_empty() {
                counter
                    .entry(line.to_vec())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
        };
        for line in output.stdout.split(|char| *char == b'\n') {
            parse_line(line);
        }
        for line in output.stderr.split(|char| *char == b'\n') {
            parse_line(line);
        }
        Self {
            counter,
            result,
            output,
        }
    }
    fn check(&mut self, line: &[u8], count: usize) {
        if let Some((_, c)) = self.counter.remove_entry(line) {
            if c != count {
                eprintln!("Stdout : {}", String::from_utf8_lossy(&self.output.stdout));
                eprintln!("Stderr : {}", String::from_utf8_lossy(&self.output.stderr));
                panic!(
                    "Check failed: {c} != {count} for line {}",
                    String::from_utf8_lossy(line)
                );
            }
        } else {
            if count != 0 {
                eprintln!("Stdout : {}", String::from_utf8_lossy(&self.output.stdout));
                eprintln!("Stderr : {}", String::from_utf8_lossy(&self.output.stderr));
                panic!(
                    "Check failed: Expected {count} line {}",
                    String::from_utf8_lossy(line)
                );
            }
        }
    }
    fn check_end(&mut self, result: TestResult) {
        assert_eq!(self.result, result);
        if self.counter.len() != 0 {
            let count = self.counter.len();
            let left_over = self
                .counter
                .drain()
                .map(|(l, _c)| format!("|{}|", String::from_utf8_lossy(&l)))
                .collect::<Vec<_>>()
                .join("\n");
            panic!("Some leftover {count}: {left_over}");
        }
    }
}

#[test]
fn test_output() {
    let output = run(None).unwrap();
    let mut dict = TestCollector::collect(output);

    dict.check(b"BUILD Number", 1);
    dict.check(b"BUILD ParamNumber", 3);
    dict.check(b"TEST test_number", 1);
    dict.check(b"TEST test_param_number", 3);
    dict.check(b"test test_number                               ... ok", 1);
    dict.check(
        b"test test_ignored_number                       ... ignored",
        1,
    );
    dict.check(b"test test_param_number[ParamNumber:5]          ... ok", 1);
    dict.check(b"test test_param_number[ParamNumber:6]          ... ok", 1);
    dict.check(b"test test_param_number[ParamNumber:42]         ... ok", 1);
    dict.check(
        b"test test_ignored_param_number[ParamNumber:5]  ... ignored",
        1,
    );
    dict.check(
        b"test test_ignored_param_number[ParamNumber:6]  ... ignored",
        1,
    );
    dict.check(
        b"test test_ignored_param_number[ParamNumber:42] ... ignored",
        1,
    );
    dict.check_end(TestResult {
        tested: 8,
        passed: 4,
        ignored: 4,
        ..Default::default()
    });
}

#[test]
fn test_output_param_only() {
    let output = run(Some(&["test_param_number"])).unwrap();
    let mut dict = TestCollector::collect(output);

    dict.check(b"BUILD Number", 0);
    dict.check(b"BUILD ParamNumber", 3);
    dict.check(b"TEST test_number", 0);
    dict.check(b"TEST test_param_number", 3);
    dict.check(b"test test_number                               ... ok", 0);
    dict.check(
        b"test test_ignored_number                       ... ignored",
        0,
    );
    dict.check(b"test test_param_number[ParamNumber:5]  ... ok", 1);
    dict.check(b"test test_param_number[ParamNumber:6]  ... ok", 1);
    dict.check(b"test test_param_number[ParamNumber:42] ... ok", 1);
    dict.check(
        b"test test_ignored_param_number[ParamNumber:5]  ... ignored",
        0,
    );
    dict.check(
        b"test test_ignored_param_number[ParamNumber:6]  ... ignored",
        0,
    );
    dict.check(
        b"test test_ignored_param_number[ParamNumber:42] ... ignored",
        0,
    );
    dict.check_end(TestResult {
        tested: 3,
        passed: 3,
        filtered_out: 5,
        ..Default::default()
    });
}

#[test]
fn test_output_ignored() {
    let output = run(Some(&["--include-ignored"])).unwrap();
    let mut dict = TestCollector::collect(output);

    dict.check(b"BUILD Number", 2);
    dict.check(b"BUILD ParamNumber", 6);
    dict.check(b"TEST test_number", 1);
    dict.check(b"TEST test_ignored_number", 1);
    dict.check(b"TEST test_param_number", 3);
    dict.check(b"TEST test_ignored_param_number", 3);
    dict.check(b"test test_number                               ... ok", 1);
    dict.check(b"test test_ignored_number                       ... ok", 1);
    dict.check(b"test test_param_number[ParamNumber:5]          ... ok", 1);
    dict.check(b"test test_param_number[ParamNumber:6]          ... ok", 1);
    dict.check(b"test test_param_number[ParamNumber:42]         ... ok", 1);
    dict.check(b"test test_ignored_param_number[ParamNumber:5]  ... ok", 1);
    dict.check(b"test test_ignored_param_number[ParamNumber:6]  ... ok", 1);
    dict.check(b"test test_ignored_param_number[ParamNumber:42] ... ok", 1);
    dict.check_end(TestResult {
        tested: 8,
        passed: 8,
        filtered_out: 0,
        ..Default::default()
    });
}

#[test]
fn test_output_ignored_only() {
    let output = run(Some(&["--ignored"])).unwrap();
    let mut dict = TestCollector::collect(output);

    dict.check(b"BUILD Number", 1);
    dict.check(b"BUILD ParamNumber", 3);
    dict.check(b"TEST test_number", 0);
    dict.check(b"TEST test_ignored_number", 1);
    dict.check(b"TEST test_param_number", 0);
    dict.check(b"TEST test_ignored_param_number", 3);
    dict.check(b"test test_number                               ... ok", 0);
    dict.check(b"test test_ignored_number                       ... ok", 1);
    dict.check(b"test test_param_number[ParamNumber:5]          ... ok", 0);
    dict.check(b"test test_param_number[ParamNumber:6]          ... ok", 0);
    dict.check(b"test test_param_number[ParamNumber:42]         ... ok", 0);
    dict.check(b"test test_ignored_param_number[ParamNumber:5]  ... ok", 1);
    dict.check(b"test test_ignored_param_number[ParamNumber:6]  ... ok", 1);
    dict.check(b"test test_ignored_param_number[ParamNumber:42] ... ok", 1);
    dict.check_end(TestResult {
        tested: 4,
        passed: 4,
        filtered_out: 4,
        ..Default::default()
    });
}

#[test]
fn test_output_list() {
    let output = run(Some(&["--list"])).unwrap();
    assert_eq!(
        output.stdout,
        b"test_number: test
test_ignored_number: test
test_param_number[ParamNumber:5]: test
test_param_number[ParamNumber:6]: test
test_param_number[ParamNumber:42]: test
test_ignored_param_number[ParamNumber:5]: test
test_ignored_param_number[ParamNumber:6]: test
test_ignored_param_number[ParamNumber:42]: test
",
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        output.stderr,
        b"",
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
