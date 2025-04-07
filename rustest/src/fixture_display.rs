use core::{clone::Clone, ops::Deref};

/// A trait to display fixtures when we have multiple combination for a test.
///
/// `FixtureDisplay` is used to provide a name for a fixture, mainly to identify test case.
pub trait FixtureDisplay {
    /// Returns the name of the fixture.
    ///
    /// # Returns
    ///
    /// The name of the fixture as a `String`.
    fn display(&self) -> String;
}

macro_rules! impl_display {
    ($($t:ty),+) => {
        $(impl FixtureDisplay for $t {
            fn display(&self) -> String {
                format!("{}", self)
            }
        })+
    };
}

impl_display!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, bool, f32, f64, char
);

impl<T: FixtureDisplay> FixtureDisplay for Box<T> {
    fn display(&self) -> String {
        self.deref().display()
    }
}

impl<T: FixtureDisplay> FixtureDisplay for Option<T> {
    fn display(&self) -> String {
        match self {
            Some(v) => v.display(),
            None => "None".to_owned(),
        }
    }
}

impl FixtureDisplay for str {
    fn display(&self) -> String {
        self.to_owned()
    }
}

impl FixtureDisplay for &str {
    fn display(&self) -> String {
        (*self).to_owned()
    }
}

impl FixtureDisplay for String {
    fn display(&self) -> String {
        self.clone()
    }
}

impl<T: FixtureDisplay> FixtureDisplay for Vec<T> {
    fn display(&self) -> String {
        let vec = self.iter().map(|v| v.display()).collect::<Vec<_>>();
        format!("[{}]", vec.join(","))
    }
}

impl FixtureDisplay for () {
    fn display(&self) -> String {
        "()".to_owned()
    }
}

macro_rules! impl_fixture_display_tuple {
    (($($types:tt),+), ($($names:ident),+)) => {

        impl< $($types),+ > FixtureDisplay for ($($types),+,)
           where
                $($types : FixtureDisplay),+ ,
        {
            fn display(&self) -> String {
                let ($($names),+, ) = self;
                $(let $names = $names.display();)+
                let vec = vec![$($names),+];
                format!("({})", vec.join(","))
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
        assert_eq!(0u8.display(), "0");
        assert_eq!(42u8.display(), "42");
        assert_eq!(0u16.display(), "0");
        assert_eq!(42u16.display(), "42");
        assert_eq!(0u32.display(), "0");
        assert_eq!(42u32.display(), "42");
        assert_eq!(0u64.display(), "0");
        assert_eq!(42u64.display(), "42");
        assert_eq!(0u128.display(), "0");
        assert_eq!(42u128.display(), "42");
        assert_eq!(0usize.display(), "0");
        assert_eq!(42usize.display(), "42");

        assert_eq!(0i8.display(), "0");
        assert_eq!(42i8.display(), "42");
        assert_eq!((-42i8).display(), "-42");
        assert_eq!(0i16.display(), "0");
        assert_eq!(42i16.display(), "42");
        assert_eq!((-42i16).display(), "-42");
        assert_eq!(0i32.display(), "0");
        assert_eq!(42i32.display(), "42");
        assert_eq!((-42i32).display(), "-42");
        assert_eq!(0i64.display(), "0");
        assert_eq!(42i64.display(), "42");
        assert_eq!((-42i64).display(), "-42");
        assert_eq!(0i128.display(), "0");
        assert_eq!(42i128.display(), "42");
        assert_eq!((-42i128).display(), "-42");
        assert_eq!(0isize.display(), "0");
        assert_eq!(42isize.display(), "42");
        assert_eq!((-42isize).display(), "-42");
    }

    #[test]
    fn test_bool() {
        assert_eq!(true.display(), "true");
        assert_eq!(false.display(), "false");
    }

    #[test]
    fn test_float() {
        assert_eq!(0.0f32.display(), "0");
        assert_eq!(1.0f32.display(), "1");
        assert_eq!(0.1f32.display(), "0.1");
        assert_eq!(0.1f32.display(), "0.1");
        assert_eq!(3.5f32.display(), "3.5");
        assert_eq!(27f32.display(), "27");
        assert_eq!((-113.75f32).display(), "-113.75");
        assert_eq!(0.0078125f32.display(), "0.0078125");
        assert_eq!(34359738368f32.display(), "34359740000");
        assert_eq!(0f32.display(), "0");
        assert_eq!((-0.0f32).display(), "-0");
        assert_eq!((-1f32).display(), "-1");
        assert_eq!(f32::NAN.display(), "NaN");
        assert_eq!(f32::INFINITY.display(), "inf");
        assert_eq!(f32::NEG_INFINITY.display(), "-inf");

        assert_eq!(0.0f64.display(), "0");
        assert_eq!(1.0f64.display(), "1");
        assert_eq!(0.1f64.display(), "0.1");
        assert_eq!(0.1f64.display(), "0.1");
        assert_eq!(3.5f64.display(), "3.5");
        assert_eq!(27f64.display(), "27");
        assert_eq!((-113.75f64).display(), "-113.75");
        assert_eq!(0.0078125f64.display(), "0.0078125");
        assert_eq!(34359738368f64.display(), "34359738368");
        assert_eq!(0f64.display(), "0");
        assert_eq!((-0.0f64).display(), "-0");
        assert_eq!((-1f64).display(), "-1");
        assert_eq!(f64::NAN.display(), "NaN");
        assert_eq!(f64::INFINITY.display(), "inf");
        assert_eq!(f64::NEG_INFINITY.display(), "-inf");
    }

    #[test]
    fn test_char() {
        assert_eq!('a'.display(), "a");
        assert_eq!('+'.display(), "+");
        assert_eq!('é'.display(), "é");
        assert_eq!('\u{0301}'.display(), "\u{301}");
        assert_eq!(char::REPLACEMENT_CHARACTER.display(), "�");
    }

    #[test]
    fn test_str() {
        assert_eq!("a".display(), "a");
        assert_eq!("+".display(), "+");
        assert_eq!("é".display(), "é");
        // This is the letter 'e' followed by a acute accent
        assert_eq!("é".display(), "e\u{301}");
        assert_eq!("\u{0065}\u{0301}".display(), "e\u{301}");
        assert_eq!("\u{0065}\u{0301}".display(), "é");
        assert_eq!("💯 love: ❤".display(), "💯 love: ❤");
    }

    #[test]
    fn test_string() {
        assert_eq!(String::from("a").display(), "a");
        assert_eq!(String::from("+").display(), "+");
        assert_eq!(String::from("é").display(), "é");
        // This is the letter 'e' followed by a acute accent
        assert_eq!(String::from("é").display(), "e\u{301}");
        assert_eq!(String::from("\u{0065}\u{0301}").display(), "e\u{301}");
        assert_eq!(String::from("\u{0065}\u{0301}").display(), "é");
        assert_eq!(String::from("💯 love: ❤").display(), "💯 love: ❤");
    }

    #[test]
    fn test_box() {
        let b = Box::new("A text");
        assert_eq!(b.display(), "A text");
        let b = Box::new(42585u32);
        assert_eq!(b.display(), "42585");
    }

    #[test]
    fn test_option() {
        assert_eq!(None::<u32>.display(), "None");
        assert_eq!(Some(42585u32).display(), "42585");
    }

    #[test]
    fn test_vec() {
        assert_eq!(Vec::<u32>::new().display(), "[]");
        assert_eq!(vec![42585u32].display(), "[42585]");
        assert_eq!(vec![4, 5].display(), "[4,5]");
    }

    #[test]
    fn test_tuple() {
        assert_eq!(().display(), "()");
        assert_eq!((5, 6).display(), "(5,6)");
        assert_eq!((5, false, "A text").display(), "(5,false,A text)");
        assert_eq!(
            (5, false, (Box::new(42), vec![5; 3])).display(),
            "(5,false,(42,[5,5,5]))"
        );
    }
}
