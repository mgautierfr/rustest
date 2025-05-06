mod other_mod;
mod sub_module_fixture;

pub use other_mod::*;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;
    use rustest::test;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert!(result == 4);
    }

    #[test]
    fn it_works_with_gtest() {
        let result = add(2, 2);
        assert_that!(result, eq(4));
    }
}

#[cfg(test)]
#[rustest::main]
fn main() {}
