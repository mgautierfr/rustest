pub fn addition(a: u32, b: u32) -> u32 {
    a + b
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rustest::test;

    #[test(params:(u32, u32, u32)=[
        (1,2,3),
        (5,6,11),
        (598318, 54876521, 55474839)
    ])]
    fn test_addition_ok(Param((a, b, expected)): Param) {
        assert_eq!(addition(a, b), expected);
    }

    #[test(params:(u32, u32, u32)=[
        (1,2,4),
        (5,6,5555),
        (598318, 54876521, 0)
    ])]
    #[xfail]
    fn test_addition_fail(Param((a, b, expected)): Param) {
        assert_eq!(addition(a, b), expected);
    }
}
