[![Crate][crate-image]][crate-link]
[![Docs][docs-image]][docs-link]
[![Status][test-action-image]][test-action-link]
[![Apache 2.0 Licensed][license-apache-image]][license-apache-link]
[![MIT Licensed][license-mit-image]][license-mit-link]

# rustest: Helps you better test programs

The rustest framework makes it easy to write small, readable tests, and can scale to support complex functional testing for applications and libraries.

Think about pytest, but for rust.

## Features

* **Fixture Management:**  Easily define and manage test fixtures, including:
 - fixture scopes (once, matrix, test, global)
 - setup and teardown functionalities
 - fixtures dependencies
* **Parametrized Tests:**  Run tests with different parameters, generating multiple test cases from a single test function.
* **Easy to use:** Define tests using standard `#[test]` attributes, providing flexibility and familiarity.


## Why a new test framework ?

When adding tests to the [waj](https://github.com/jubako/waj) crate I was needed to have a global (static) fixture with teardown. Something like :

```rust
#[fixture]
fn Command() -> std::process::Command {
    std::process::Command::new("bash")
        .stdout(Stdio::piped())
        .arg("-c")
        .arg("while true; do sleep 1; done")
}

#[fixture(scope=global, teardown=|v| v.kill())]
fn RunningProcess(cmd: Command) -> std::io::Result<Box<std::process::Child>> {
    Ok(Box::new(cmd.spawn()?))
}
```

I have found no test framework allowing to have teardown on global fixtures.
Storing the running process in a static LazyLock allow to have a simple "fixture" but, as statics are not drop, no
teardown either.

So I had to implement it, and I finish with a "full" test framework. This became this crate.

This crate, and its API, take inspirations from:
- [pytest](https://docs.pytest.org/en/stable/)
- [rstest](https://crates.io/crates/rstest)
- [test-case](https://crates.io/crates/test-case)
- [libtest-mimic-collect](https://crates.io/crates/libtest-mimic-collect)

It is based on [libtest-mimic](https://crates.io/crates/libtest-mimic) to run the tests.

## Getting Started

### Setup

Add rustest to your `Cargo.toml` file:

```
$ cargo add --dev rustest
```

Rustest comes with its own test harness, so you must deactivate the default one in Cargo.toml:

```toml
# In Cargo.toml

[[test]]
name = "test_name" # for a test located at "tests/test_name.rs"
harness = false

[[test]]
name = "other_test" # for a test located at "tests/other_test.rs"
harness = false

# For unit test, you also need to deactivate harness for lib
[lib]
harness = false
```

You also need to add a main function in each of your integration tests. To do so add an empty main function and
mark it with `#[rustest::main]` attribute.
For unit testing, add the main function at end of you `lib.rs` file, under a `cfg(test)` flag:

```rust
#[cfg(test)]
#[rustest::main]
fn main () {}
```


## Usage Examples

Here are some examples demonstrating rustest's key features.
The file `tests/test.rs` shows all rustest's features and acts as examples and documentation.

**Simple Test:**

Simple tests are as simple as with standard test library. Don't forget to define the main function.

```rust
use rustest::{test, main};

#[test]
fn simple_test() {
    assert_eq!(5*6, 30)
}

#[main]
fn main() {}
```

**Failing Tests**

Tests can be marked as expecting to fail. Either with `#[xfail]` attribute or `#[test(xfail)]`

```rust
use rustest::{test, main};

#[test(xfail)]
fn failing_test() {
    assert_eq!(5*6, 31)
}

#[test]
#[xfail]
fn failing_test_bis() {
    assert_eq!(5*6, 31)
}

#[main]
fn main() {}
```

**Fixture Example:**

You can define any fixtures using the `#[fixture]` attribute on a function.

```rust
// This define a fixture name ANumber which can be deref to u32.
// The function will be called to everytime we need a `ANumber` to populate the fixture
#[fixture]
fn ANumber() -> u32 {
    5
}

// Fixtures are requested by their types.
#[test]
fn test_with_fixture(number: ANumber) {
    assert_eq!(*number, 5);
}
```

**Fixture teardown**

You can define a teardown function to be called when the fixture is drop:

```rust
#[fixture(teardown:|v| println!("Teardown with value {}", v))]
fn TeardownNumber() -> u32 {
    5
}

// Print "Teardown with value 5" at end of test.
#[test]
fn test_with_teardown_fixture(number: TeardownNumber) {
    assert_eq!(*number, 5);
}    
```

**Fixture Scope:**

By default, fixtures are created each time they are requested.

```rust
static GLOBAL_COUNTER: AtomicU32 = AtomicU32::new(0);

#[fixture]
fn Counter() -> u32 {
    GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[test]
fn test_counter(counter1: Counter, counter2: Counter) {
    assert_ne!(*counter1, *counter2);
    assert_eq!(*counter1, 0);
    assert_eq!(*counter2, 1);
}
```

With `scope=test`, we create only one fixture (of each type) per test.

This will create twice the TestCounter
    
```rust
#[fixture(scope=test)]
fn TestCounter() -> u32 {
    GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[test]
fn test_local_counter1(counter1: TestCounter, counter2: TestCounter) {
    assert_eq!(*counter1, *counter2);
    assert_eq!(*counter1, 2);
    assert_eq!(*counter2, 2);
}

#[test]
fn test_local_counter2(counter1: TestCounter, counter2: TestCounter) {
    assert_eq!(*counter1, *counter2);
    assert_eq!(*counter1, 3);
    assert_eq!(*counter2, 3);
}

```

A global scope make the fixture created only once:
    
```rust
#[fixture(scope=global)]
fn GlobalCounter() -> u32 {
    GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[test]
fn test_global_counter1(counter1: GlobalCounter, counter2: GlobalCounter) {
    assert_eq!(*counter1, *counter2);
    assert_eq!(*counter1, 4);
    assert_eq!(*counter2, 4);
}

#[test]
fn test_global_counter2(counter1: GlobalCounter, counter2: GlobalCounter) {
    assert_eq!(*counter1, *counter2);
    assert_eq!(*counter1, 4);
    assert_eq!(*counter2, 4);
}
```

**Parametrized Fixture:**

```rust
#[fixture(params:u32=[1, 5])]
fn ParametrizedFixture(p: Param) -> u32 {
    *p
}

#[test]
fn test_parametrized_fixture(param: ParametrizedFixture) {
    assert!([1, 5].contains(&param));
}
```

**Fixtures can use fixtures**

```rust
fn ANumberAsString(number: ANumber) -> String {
    format!("This is a number : {}", *number)
}

#[test]
fn test_number_string(text: ANumberAsString) {
    assert_eq!(*text, "This is a number : 5")
}
```

**Fixtures can be Generic**

```rust
#[fixture]
fn NumberAsString<Source>(number: Source) -> String
where
    Source: rustest::Fixture<Type =u32>
{
    format!("This is a number : {}", *number)
}

#[fixture]
fn TheNumber6() -> u32 {
    6
}

#[test]
fn test_number_string_5(text: NumberAsString<ANumber>) {
    assert_eq!(*text, "This is a number : 5")
}

#[test]
fn test_number_string_6(text: NumberAsString<TheNumber6>) {
    assert_eq!(*text, "This is a number : 6")
}
```


**Running Tests:**

Execute your tests using the standard `cargo test` command.  Rustest uses `libtest-mimic` which provides a compatible interface for running your tests.

```bash
cargo test
```

## Contributing

Rustest is pretty young. Issue reports and PR are welcomed !


## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](/LICENSE-APACHE) or [license-apache-link](http://www.apache.org/licenses/LICENSE-2.0))

* MIT license [LICENSE-MIT](/LICENSE-MIT) or [license-MIT-link](http://opensource.org/licenses/MIT) at your option.


[//]: # (links)

[crate-image]: https://img.shields.io/crates/v/rustest.svg
[crate-link]: https://crates.io/crates/rustest
[docs-image]: https://docs.rs/rustest/badge.svg
[docs-link]: https://docs.rs/rustest/
[test-action-image]: https://github.com/mgautierfr/rustest/workflows/Cargo%20Build%20&%20Test/badge.svg
[test-action-link]: https://github.com/mgautierfr/rustest/actions/workflows/ci.yml?query=workflow%3ACargo
[license-apache-image]: https://img.shields.io/badge/license-Apache2.0-blue.svg
[license-mit-image]: https://img.shields.io/badge/license-MIT-blue.svg
[license-apache-link]: http://www.apache.org/licenses/LICENSE-2.0
[license-MIT-link]: http://opensource.org/licenses/MIT

