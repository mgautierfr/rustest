use core::{clone::Clone, ops::Deref};
use std::sync::Mutex;

/// A trait to display fixtures when we have multiple combination for a test.
///
/// `FixtureDisplay` is used to provide a name for a fixture, mainly to identify test case.
pub trait FixtureDisplay {
    /// Returns the name of the fixture.
    ///
    /// # Returns
    ///
    /// The name of the fixture as a `String`.
    fn display(&self) -> Option<String>;
}

macro_rules! impl_display {
    ($($t:ty),+) => {
        $(impl FixtureDisplay for $t {
            fn display(&self) -> Option<String> {
                Some(format!("{}", self))
            }
        })+
    };
}

impl_display!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, bool, f32, f64, char
);

impl<T: FixtureDisplay> FixtureDisplay for Box<T> {
    fn display(&self) -> Option<String> {
        self.deref().display()
    }
}

impl<T: FixtureDisplay> FixtureDisplay for Option<T> {
    fn display(&self) -> Option<String> {
        match self {
            Some(v) => v.display(),
            None => Some("None".to_owned()),
        }
    }
}

impl FixtureDisplay for str {
    fn display(&self) -> Option<String> {
        Some(self.to_owned())
    }
}

impl FixtureDisplay for &str {
    fn display(&self) -> Option<String> {
        Some((*self).to_owned())
    }
}

impl FixtureDisplay for String {
    fn display(&self) -> Option<String> {
        Some(self.clone())
    }
}

impl<T: FixtureDisplay> FixtureDisplay for Vec<T> {
    fn display(&self) -> Option<String> {
        let vec = self.iter().filter_map(|v| v.display()).collect::<Vec<_>>();
        if vec.is_empty() {
            None
        } else {
            Some(format!("[{}]", vec.join(",")))
        }
    }
}

impl<T: FixtureDisplay> FixtureDisplay for Mutex<T> {
    fn display(&self) -> Option<String> {
        self.lock().unwrap().display()
    }
}

impl FixtureDisplay for () {
    fn display(&self) -> Option<String> {
        None
    }
}

macro_rules! impl_fixture_display_tuple {
    (($($types:tt),+), ($($names:ident),+)) => {

        impl< $($types),+ > FixtureDisplay for ($($types),+,)
           where
                $($types : FixtureDisplay),+ ,
        {
            fn display(&self) -> Option<String> {
                let ($($names),+, ) = self;
                $(let $names = $names.display();)+
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

impl_fixture_display_tuple!((F0), (f0));
impl_fixture_display_tuple!((F0, F1), (f0, f1));
impl_fixture_display_tuple!((F0, F1, F2), (f0, f1, f2));
impl_fixture_display_tuple!((F0, F1, F2, F3), (f0, f1, f2, f3));
impl_fixture_display_tuple!((F0, F1, F2, F3, F4), (f0, f1, f2, f3, f4));
impl_fixture_display_tuple!((F0, F1, F2, F3, F4, F5), (f0, f1, f2, f3, f4, f5));
impl_fixture_display_tuple!((F0, F1, F2, F3, F4, F5, F6), (f0, f1, f2, f3, f4, f5, f6));
impl_fixture_display_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7),
    (f0, f1, f2, f3, f4, f5, f6, f7)
);
impl_fixture_display_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8)
);
impl_fixture_display_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
);
impl_fixture_display_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10)
);
impl_fixture_display_tuple!(
    (F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11),
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11)
);

#[cfg(test)]
mod tests {
    use core::assert_eq;

    use super::*;

    #[test]
    fn test_integers() {
        assert_eq!(0u8.display(), Some("0".into()));
        assert_eq!(42u8.display(), Some("42".into()));
        assert_eq!(0u16.display(), Some("0".into()));
        assert_eq!(42u16.display(), Some("42".into()));
        assert_eq!(0u32.display(), Some("0".into()));
        assert_eq!(42u32.display(), Some("42".into()));
        assert_eq!(0u64.display(), Some("0".into()));
        assert_eq!(42u64.display(), Some("42".into()));
        assert_eq!(0u128.display(), Some("0".into()));
        assert_eq!(42u128.display(), Some("42".into()));
        assert_eq!(0usize.display(), Some("0".into()));
        assert_eq!(42usize.display(), Some("42".into()));

        assert_eq!(0i8.display(), Some("0".into()));
        assert_eq!(42i8.display(), Some("42".into()));
        assert_eq!((-42i8).display(), Some("-42".into()));
        assert_eq!(0i16.display(), Some("0".into()));
        assert_eq!(42i16.display(), Some("42".into()));
        assert_eq!((-42i16).display(), Some("-42".into()));
        assert_eq!(0i32.display(), Some("0".into()));
        assert_eq!(42i32.display(), Some("42".into()));
        assert_eq!((-42i32).display(), Some("-42".into()));
        assert_eq!(0i64.display(), Some("0".into()));
        assert_eq!(42i64.display(), Some("42".into()));
        assert_eq!((-42i64).display(), Some("-42".into()));
        assert_eq!(0i128.display(), Some("0".into()));
        assert_eq!(42i128.display(), Some("42".into()));
        assert_eq!((-42i128).display(), Some("-42".into()));
        assert_eq!(0isize.display(), Some("0".into()));
        assert_eq!(42isize.display(), Some("42".into()));
        assert_eq!((-42isize).display(), Some("-42".into()));
    }

    #[test]
    fn test_bool() {
        assert_eq!(true.display(), Some("true".into()));
        assert_eq!(false.display(), Some("false".into()));
    }

    #[test]
    fn test_float() {
        assert_eq!(0.0f32.display(), Some("0".into()));
        assert_eq!(1.0f32.display(), Some("1".into()));
        assert_eq!(0.1f32.display(), Some("0.1".into()));
        assert_eq!(0.1f32.display(), Some("0.1".into()));
        assert_eq!(3.5f32.display(), Some("3.5".into()));
        assert_eq!(27f32.display(), Some("27".into()));
        assert_eq!((-113.75f32).display(), Some("-113.75".into()));
        assert_eq!(0.0078125f32.display(), Some("0.0078125".into()));
        assert_eq!(34359738368f32.display(), Some("34359740000".into()));
        assert_eq!(0f32.display(), Some("0".into()));
        assert_eq!((-0.0f32).display(), Some("-0".into()));
        assert_eq!((-1f32).display(), Some("-1".into()));
        assert_eq!(f32::NAN.display(), Some("NaN".into()));
        assert_eq!(f32::INFINITY.display(), Some("inf".into()));
        assert_eq!(f32::NEG_INFINITY.display(), Some("-inf".into()));

        assert_eq!(0.0f64.display(), Some("0".into()));
        assert_eq!(1.0f64.display(), Some("1".into()));
        assert_eq!(0.1f64.display(), Some("0.1".into()));
        assert_eq!(0.1f64.display(), Some("0.1".into()));
        assert_eq!(3.5f64.display(), Some("3.5".into()));
        assert_eq!(27f64.display(), Some("27".into()));
        assert_eq!((-113.75f64).display(), Some("-113.75".into()));
        assert_eq!(0.0078125f64.display(), Some("0.0078125".into()));
        assert_eq!(34359738368f64.display(), Some("34359738368".into()));
        assert_eq!(0f64.display(), Some("0".into()));
        assert_eq!((-0.0f64).display(), Some("-0".into()));
        assert_eq!((-1f64).display(), Some("-1".into()));
        assert_eq!(f64::NAN.display(), Some("NaN".into()));
        assert_eq!(f64::INFINITY.display(), Some("inf".into()));
        assert_eq!(f64::NEG_INFINITY.display(), Some("-inf".into()));
    }

    #[test]
    fn test_char() {
        assert_eq!('a'.display(), Some("a".into()));
        assert_eq!('+'.display(), Some("+".into()));
        assert_eq!('é'.display(), Some("é".into()));
        assert_eq!('\u{0301}'.display(), Some("\u{301}".into()));
        assert_eq!(char::REPLACEMENT_CHARACTER.display(), Some("�".into()));
    }

    #[test]
    fn test_str() {
        assert_eq!("a".display(), Some("a".into()));
        assert_eq!("+".display(), Some("+".into()));
        assert_eq!("é".display(), Some("é".into()));
        // This is the letter 'e' followed by a acute accent
        assert_eq!("é".display(), Some("e\u{301}".into()));
        assert_eq!("\u{0065}\u{0301}".display(), Some("e\u{301}".into()));
        assert_eq!("\u{0065}\u{0301}".display(), Some("é".into()));
        assert_eq!("💯 love: ❤".display(), Some("💯 love: ❤".into()));
    }

    #[test]
    fn test_string() {
        assert_eq!(String::from("a").display(), Some("a".into()));
        assert_eq!(String::from("+").display(), Some("+".into()));
        assert_eq!(String::from("é").display(), Some("é".into()));
        // This is the letter 'e' followed by a acute accent
        assert_eq!(String::from("é").display(), Some("e\u{301}".into()));
        assert_eq!(
            String::from("\u{0065}\u{0301}").display(),
            Some("e\u{301}".into())
        );
        assert_eq!(String::from("\u{0065}\u{0301}").display(), Some("é".into()));
        assert_eq!(
            String::from("💯 love: ❤").display(),
            Some("💯 love: ❤".into())
        );
    }

    #[test]
    fn test_box() {
        let b = Box::new("A text");
        assert_eq!(b.display(), Some("A text".into()));
        let b = Box::new(42585u32);
        assert_eq!(b.display(), Some("42585".into()));
    }

    #[test]
    fn test_option() {
        assert_eq!(None::<u32>.display(), Some("None".into()));
        assert_eq!(Some(42585u32).display(), Some("42585".into()));
    }

    #[test]
    fn test_vec() {
        assert_eq!(Vec::<u32>::new().display(), None);
        assert_eq!(vec![42585u32].display(), Some("[42585]".into()));
        assert_eq!(vec![4, 5].display(), Some("[4,5]".into()));
    }

    #[test]
    fn test_mutex() {
        assert_eq!(Mutex::new("a text").display(), Some("a text".into()));
    }

    #[test]
    fn test_tuple() {
        assert_eq!(().display(), None);
        assert_eq!((5, 6).display(), Some("(5,6)".into()));
        assert_eq!(
            (5, false, "A text").display(),
            Some("(5,false,A text)".into())
        );
        assert_eq!(
            (5, false, (Box::new(42), vec![5; 3])).display(),
            Some("(5,false,(42,[5,5,5]))".into())
        );
    }
}
