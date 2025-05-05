use core::{clone::Clone, ops::Deref};
use std::sync::Mutex;

/// A trait to get the name of a test when we have multiple combination.
///
/// `TestName` is used to provide a name for a test or a part of it.
pub trait TestName {
    /// Returns the name of the test.
    ///
    /// # Returns
    ///
    /// The name of the test as a `String`.
    fn name(&self) -> Option<String>;
}

macro_rules! impl_test_name {
    ($($t:ty),+) => {
        $(impl TestName for $t {
            fn name(&self) -> Option<String> {
                Some(format!("{}", self))
            }
        })+
    };
}

impl_test_name!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, bool, f32, f64, char
);

impl<T: TestName> TestName for Box<T> {
    fn name(&self) -> Option<String> {
        self.deref().name()
    }
}

impl<T: TestName> TestName for Option<T> {
    fn name(&self) -> Option<String> {
        match self {
            Some(v) => v.name(),
            None => Some("None".to_owned()),
        }
    }
}

impl TestName for str {
    fn name(&self) -> Option<String> {
        Some(self.to_owned())
    }
}

impl TestName for &str {
    fn name(&self) -> Option<String> {
        Some((*self).to_owned())
    }
}

impl TestName for String {
    fn name(&self) -> Option<String> {
        Some(self.clone())
    }
}

impl<T: TestName> TestName for Vec<T> {
    fn name(&self) -> Option<String> {
        let vec = self.iter().filter_map(|v| v.name()).collect::<Vec<_>>();
        if vec.is_empty() {
            None
        } else {
            Some(format!("[{}]", vec.join(",")))
        }
    }
}

impl<T: TestName> TestName for Mutex<T> {
    fn name(&self) -> Option<String> {
        self.lock().unwrap().name()
    }
}

impl TestName for () {
    fn name(&self) -> Option<String> {
        None
    }
}

macro_rules! impl_fixture_name_tuple {
    (($($types:tt),+), ($($names:ident),+)) => {

        impl< $($types),+ > TestName for ($($types),+,)
           where
                $($types : TestName),+ ,
        {
            fn name(&self) -> Option<String> {
                let ($($names),+, ) = self;
                $(let $names = $names.name();)+
                let vec = vec![$($names),+].into_iter().filter_map(|d|d).collect::<Vec<_>>();
                if vec.is_empty() {
                    None
                } else {
                    Some(format!("({})", vec.join(",")))
                }
            }
        }
    }
}

impl_fixture_name_tuple!((F0), (f0));
impl_fixture_name_tuple!((F0, F1), (f0, f1));
impl_fixture_name_tuple!((F0, F1, F2), (f0, f1, f2));
impl_fixture_name_tuple!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_fixture_name_tuple!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_fixture_name_tuple!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_fixture_name_tuple!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_fixture_name_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_fixture_name_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_fixture_name_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_fixture_name_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_fixture_name_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

#[cfg(test)]
mod tests {
    use core::assert_eq;

    use super::*;

    #[test]
    fn test_integers() {
        assert_eq!(0u8.name(), Some("0".into()));
        assert_eq!(42u8.name(), Some("42".into()));
        assert_eq!(0u16.name(), Some("0".into()));
        assert_eq!(42u16.name(), Some("42".into()));
        assert_eq!(0u32.name(), Some("0".into()));
        assert_eq!(42u32.name(), Some("42".into()));
        assert_eq!(0u64.name(), Some("0".into()));
        assert_eq!(42u64.name(), Some("42".into()));
        assert_eq!(0u128.name(), Some("0".into()));
        assert_eq!(42u128.name(), Some("42".into()));
        assert_eq!(0usize.name(), Some("0".into()));
        assert_eq!(42usize.name(), Some("42".into()));

        assert_eq!(0i8.name(), Some("0".into()));
        assert_eq!(42i8.name(), Some("42".into()));
        assert_eq!((-42i8).name(), Some("-42".into()));
        assert_eq!(0i16.name(), Some("0".into()));
        assert_eq!(42i16.name(), Some("42".into()));
        assert_eq!((-42i16).name(), Some("-42".into()));
        assert_eq!(0i32.name(), Some("0".into()));
        assert_eq!(42i32.name(), Some("42".into()));
        assert_eq!((-42i32).name(), Some("-42".into()));
        assert_eq!(0i64.name(), Some("0".into()));
        assert_eq!(42i64.name(), Some("42".into()));
        assert_eq!((-42i64).name(), Some("-42".into()));
        assert_eq!(0i128.name(), Some("0".into()));
        assert_eq!(42i128.name(), Some("42".into()));
        assert_eq!((-42i128).name(), Some("-42".into()));
        assert_eq!(0isize.name(), Some("0".into()));
        assert_eq!(42isize.name(), Some("42".into()));
        assert_eq!((-42isize).name(), Some("-42".into()));
    }

    #[test]
    fn test_bool() {
        assert_eq!(true.name(), Some("true".into()));
        assert_eq!(false.name(), Some("false".into()));
    }

    #[test]
    fn test_float() {
        assert_eq!(0.0f32.name(), Some("0".into()));
        assert_eq!(1.0f32.name(), Some("1".into()));
        assert_eq!(0.1f32.name(), Some("0.1".into()));
        assert_eq!(0.1f32.name(), Some("0.1".into()));
        assert_eq!(3.5f32.name(), Some("3.5".into()));
        assert_eq!(27f32.name(), Some("27".into()));
        assert_eq!((-113.75f32).name(), Some("-113.75".into()));
        assert_eq!(0.0078125f32.name(), Some("0.0078125".into()));
        assert_eq!(34359738368f32.name(), Some("34359740000".into()));
        assert_eq!(0f32.name(), Some("0".into()));
        assert_eq!((-0.0f32).name(), Some("-0".into()));
        assert_eq!((-1f32).name(), Some("-1".into()));
        assert_eq!(f32::NAN.name(), Some("NaN".into()));
        assert_eq!(f32::INFINITY.name(), Some("inf".into()));
        assert_eq!(f32::NEG_INFINITY.name(), Some("-inf".into()));

        assert_eq!(0.0f64.name(), Some("0".into()));
        assert_eq!(1.0f64.name(), Some("1".into()));
        assert_eq!(0.1f64.name(), Some("0.1".into()));
        assert_eq!(0.1f64.name(), Some("0.1".into()));
        assert_eq!(3.5f64.name(), Some("3.5".into()));
        assert_eq!(27f64.name(), Some("27".into()));
        assert_eq!((-113.75f64).name(), Some("-113.75".into()));
        assert_eq!(0.0078125f64.name(), Some("0.0078125".into()));
        assert_eq!(34359738368f64.name(), Some("34359738368".into()));
        assert_eq!(0f64.name(), Some("0".into()));
        assert_eq!((-0.0f64).name(), Some("-0".into()));
        assert_eq!((-1f64).name(), Some("-1".into()));
        assert_eq!(f64::NAN.name(), Some("NaN".into()));
        assert_eq!(f64::INFINITY.name(), Some("inf".into()));
        assert_eq!(f64::NEG_INFINITY.name(), Some("-inf".into()));
    }

    #[test]
    fn test_char() {
        assert_eq!('a'.name(), Some("a".into()));
        assert_eq!('+'.name(), Some("+".into()));
        assert_eq!('é'.name(), Some("é".into()));
        assert_eq!('\u{0301}'.name(), Some("\u{301}".into()));
        assert_eq!(char::REPLACEMENT_CHARACTER.name(), Some("�".into()));
    }

    #[test]
    fn test_str() {
        assert_eq!("a".name(), Some("a".into()));
        assert_eq!("+".name(), Some("+".into()));
        assert_eq!("é".name(), Some("é".into()));
        // This is the letter 'e' followed by a acute accent
        assert_eq!("é".name(), Some("e\u{301}".into()));
        assert_eq!("\u{0065}\u{0301}".name(), Some("e\u{301}".into()));
        assert_eq!("\u{0065}\u{0301}".name(), Some("é".into()));
        assert_eq!("💯 love: ❤".name(), Some("💯 love: ❤".into()));
    }

    #[test]
    fn test_string() {
        assert_eq!(String::from("a").name(), Some("a".into()));
        assert_eq!(String::from("+").name(), Some("+".into()));
        assert_eq!(String::from("é").name(), Some("é".into()));
        // This is the letter 'e' followed by a acute accent
        assert_eq!(String::from("é").name(), Some("e\u{301}".into()));
        assert_eq!(
            String::from("\u{0065}\u{0301}").name(),
            Some("e\u{301}".into())
        );
        assert_eq!(String::from("\u{0065}\u{0301}").name(), Some("é".into()));
        assert_eq!(String::from("💯 love: ❤").name(), Some("💯 love: ❤".into()));
    }

    #[test]
    fn test_box() {
        let b = Box::new("A text");
        assert_eq!(b.name(), Some("A text".into()));
        let b = Box::new(42585u32);
        assert_eq!(b.name(), Some("42585".into()));
    }

    #[test]
    fn test_option() {
        assert_eq!(None::<u32>.name(), Some("None".into()));
        assert_eq!(Some(42585u32).name(), Some("42585".into()));
    }

    #[test]
    fn test_vec() {
        assert_eq!(Vec::<u32>::new().name(), None);
        assert_eq!(vec![42585u32].name(), Some("[42585]".into()));
        assert_eq!(vec![4, 5].name(), Some("[4,5]".into()));
    }

    #[test]
    fn test_mutex() {
        assert_eq!(Mutex::new("a text").name(), Some("a text".into()));
    }

    #[test]
    fn test_tuple() {
        assert_eq!(().name(), None);
        assert_eq!((5, 6).name(), Some("(5,6)".into()));
        assert_eq!((5, false, "A text").name(), Some("(5,false,A text)".into()));
        assert_eq!(
            (5, false, (Box::new(42), vec![5; 3])).name(),
            Some("(5,false,(42,[5,5,5]))".into())
        );
    }
}
