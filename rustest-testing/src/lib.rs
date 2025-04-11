mod other_mod;

pub use other_mod::*;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustest::test;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert!(result == 4);
    }
}

#[cfg(test)]
#[rustest::main]
fn main() {}
