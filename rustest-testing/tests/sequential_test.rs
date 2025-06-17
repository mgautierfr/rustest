use googletest::prelude::*;
use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;

#[test]
fn test_sequential() {
    let stderr = Command::new(env!("CARGO_BIN_EXE_sequential_test"))
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
        .stderr;
    let stderr = String::from_utf8_lossy(&stderr);

    let mut timings = HashMap::new();

    let lines = stderr.lines();
    let lines: Vec<_> = lines.collect();

    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            assert_that!(*line, eq("creating new client"));
            continue;
        } else if i == lines.len() - 1 {
            assert_that!(*line, eq("dropping client"));
            continue;
        }
        let parts = line.split(' ').collect::<Vec<_>>();
        assert_that!(parts.len(), eq(2), "line = {:?}", parts);
        let task = u16::from_str(parts[0]).unwrap();
        let timestamp = u64::from_str(parts[1]).unwrap();

        timings.entry(task).or_insert(Vec::new()).push(timestamp);
    }

    let start_first = timings.get(&1).unwrap().iter().min().unwrap();
    let end_first = timings.get(&1).unwrap().iter().max().unwrap();
    let start_second = timings.get(&2).unwrap().iter().min().unwrap();
    let end_second = timings.get(&2).unwrap().iter().max().unwrap();

    // there's no guaranteed order in which tests get executed, thus we have to check which one started first
    if start_first < start_second {
        assert_that!(
            start_second,
            ge(end_first),
            "test1 started first. timings: {timings:#?}"
        );
    } else {
        assert_that!(
            start_first,
            ge(end_second),
            "test2 started first. timings: {timings:#?}"
        );
    }
}
