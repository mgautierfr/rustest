#[cfg(test)]
pub(crate) mod tests {
    use rustest::test;

    mod sub_module {
        #[rustest::fixture(params:u32=[5,6])]
        pub fn ANumber(Param(n): Param) -> u32 {
            n
        }
    }

    #[test]
    fn test_number(n: sub_module::ANumber) {
        assert!([5, 6].contains(&n))
    }
}
