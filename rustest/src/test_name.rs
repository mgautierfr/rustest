use std::sync::Mutex;

#[doc(hidden)]
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

/// A trait to get the name of a param when we have multiple combination.
///
/// `ParamName` is used to provide a name for a test.
pub trait ParamName {
    /// Returns the name of the parameter.
    ///
    /// # Returns
    ///
    /// The name of the param as a `String`.
    fn param_name(&self) -> String;
}

macro_rules! impl_test_name {
    ($($t:ty),+) => {
        $(impl ParamName for $t {
            fn param_name(&self) -> String {
                format!("{}", self)
            }
        })+
    };
}

impl_test_name!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, bool, f32, f64, char
);

impl<T: ParamName> ParamName for Box<T> {
    fn param_name(&self) -> String {
        use std::ops::Deref;
        self.deref().param_name()
    }
}

impl<T: ParamName> ParamName for Option<T> {
    fn param_name(&self) -> String {
        match self {
            Some(v) => v.param_name(),
            None => "None".to_owned(),
        }
    }
}

pub trait ToParamName<T> {
    fn into(self) -> (T, String);
}

impl<T> ToParamName<T> for T
where
    T: ParamName,
{
    fn into(self) -> (T, String) {
        let name = self.param_name();
        (self, name)
    }
}

impl<T> ToParamName<T> for (T, String) {
    fn into(self) -> (T, String) {
        self
    }
}

impl<T> ToParamName<T> for (T, &str) {
    fn into(self) -> (T, String) {
        (self.0, self.1.to_owned())
    }
}

impl ParamName for str {
    fn param_name(&self) -> String {
        self.to_owned()
    }
}

impl ParamName for &str {
    fn param_name(&self) -> String {
        (*self).to_owned()
    }
}

impl ParamName for String {
    fn param_name(&self) -> String {
        self.clone()
    }
}

impl<T: ParamName> ParamName for Vec<T> {
    fn param_name(&self) -> String {
        let vec = self.iter().map(|v| v.param_name()).collect::<Vec<_>>();
        format!("[{}]", vec.join(","))
    }
}

impl<T: ParamName> ParamName for Mutex<T> {
    fn param_name(&self) -> String {
        self.lock().unwrap().param_name()
    }
}

macro_rules! impl_fixture_name_tuple {
    (($($types:tt),+), ($($names:ident),+)) => {

        impl< $($types),+ > ParamName for ($($types),+,)
           where
                $($types : ParamName),+ ,
        {
            fn param_name(&self) -> String {
                let ($($names),+, ) = self;
                $(let $names = $names.param_name();)+
                let vec = vec![$($names),+].into_iter().collect::<Vec<_>>();
                format!("({})", vec.join(","))
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
        assert_eq!(0u8.param_name(), "0".to_owned());
        assert_eq!(42u8.param_name(), "42".to_owned());
        assert_eq!(0u16.param_name(), "0".to_owned());
        assert_eq!(42u16.param_name(), "42".to_owned());
        assert_eq!(0u32.param_name(), "0".to_owned());
        assert_eq!(42u32.param_name(), "42".to_owned());
        assert_eq!(0u64.param_name(), "0".to_owned());
        assert_eq!(42u64.param_name(), "42".to_owned());
        assert_eq!(0u128.param_name(), "0".to_owned());
        assert_eq!(42u128.param_name(), "42".to_owned());
        assert_eq!(0usize.param_name(), "0".to_owned());
        assert_eq!(42usize.param_name(), "42".to_owned());

        assert_eq!(0i8.param_name(), "0".to_owned());
        assert_eq!(42i8.param_name(), "42".to_owned());
        assert_eq!((-42i8).param_name(), "-42".to_owned());
        assert_eq!(0i16.param_name(), "0".to_owned());
        assert_eq!(42i16.param_name(), "42".to_owned());
        assert_eq!((-42i16).param_name(), "-42".to_owned());
        assert_eq!(0i32.param_name(), "0".to_owned());
        assert_eq!(42i32.param_name(), "42".to_owned());
        assert_eq!((-42i32).param_name(), "-42".to_owned());
        assert_eq!(0i64.param_name(), "0".to_owned());
        assert_eq!(42i64.param_name(), "42".to_owned());
        assert_eq!((-42i64).param_name(), "-42".to_owned());
        assert_eq!(0i128.param_name(), "0".to_owned());
        assert_eq!(42i128.param_name(), "42".to_owned());
        assert_eq!((-42i128).param_name(), "-42".to_owned());
        assert_eq!(0isize.param_name(), "0".to_owned());
        assert_eq!(42isize.param_name(), "42".to_owned());
        assert_eq!((-42isize).param_name(), "-42".to_owned());
    }

    #[test]
    fn test_bool() {
        assert_eq!(true.param_name(), "true".to_owned());
        assert_eq!(false.param_name(), "false".to_owned());
    }

    #[test]
    fn test_float() {
        assert_eq!(0.0f32.param_name(), "0".to_owned());
        assert_eq!(1.0f32.param_name(), "1".to_owned());
        assert_eq!(0.1f32.param_name(), "0.1".to_owned());
        assert_eq!(0.1f32.param_name(), "0.1".to_owned());
        assert_eq!(3.5f32.param_name(), "3.5".to_owned());
        assert_eq!(27f32.param_name(), "27".to_owned());
        assert_eq!((-113.75f32).param_name(), "-113.75".to_owned());
        assert_eq!(0.0078125f32.param_name(), "0.0078125".to_owned());
        assert_eq!(34359738368f32.param_name(), "34359740000".to_owned());
        assert_eq!(0f32.param_name(), "0".to_owned());
        assert_eq!((-0.0f32).param_name(), "-0".to_owned());
        assert_eq!((-1f32).param_name(), "-1".to_owned());
        assert_eq!(f32::NAN.param_name(), "NaN".to_owned());
        assert_eq!(f32::INFINITY.param_name(), "inf".to_owned());
        assert_eq!(f32::NEG_INFINITY.param_name(), "-inf".to_owned());

        assert_eq!(0.0f64.param_name(), "0".to_owned());
        assert_eq!(1.0f64.param_name(), "1".to_owned());
        assert_eq!(0.1f64.param_name(), "0.1".to_owned());
        assert_eq!(0.1f64.param_name(), "0.1".to_owned());
        assert_eq!(3.5f64.param_name(), "3.5".to_owned());
        assert_eq!(27f64.param_name(), "27".to_owned());
        assert_eq!((-113.75f64).param_name(), "-113.75".to_owned());
        assert_eq!(0.0078125f64.param_name(), "0.0078125".to_owned());
        assert_eq!(34359738368f64.param_name(), "34359738368".to_owned());
        assert_eq!(0f64.param_name(), "0".to_owned());
        assert_eq!((-0.0f64).param_name(), "-0".to_owned());
        assert_eq!((-1f64).param_name(), "-1".to_owned());
        assert_eq!(f64::NAN.param_name(), "NaN".to_owned());
        assert_eq!(f64::INFINITY.param_name(), "inf".to_owned());
        assert_eq!(f64::NEG_INFINITY.param_name(), "-inf".to_owned());
    }

    #[test]
    fn test_char() {
        assert_eq!('a'.param_name(), "a".to_owned());
        assert_eq!('+'.param_name(), "+".to_owned());
        assert_eq!('é'.param_name(), "é".to_owned());
        assert_eq!('\u{0301}'.param_name(), "\u{301}".to_owned());
        assert_eq!(char::REPLACEMENT_CHARACTER.param_name(), "�".to_owned());
    }

    #[test]
    fn test_str() {
        assert_eq!("a".param_name(), "a".to_owned());
        assert_eq!("+".param_name(), "+".to_owned());
        assert_eq!("é".param_name(), "é".to_owned());
        // This is the letter 'e' followed by a acute accent
        assert_eq!("é".param_name(), "e\u{301}".to_owned());
        assert_eq!("\u{0065}\u{0301}".param_name(), "e\u{301}".to_owned());
        assert_eq!("\u{0065}\u{0301}".param_name(), "é".to_owned());
        assert_eq!("💯 love: ❤".param_name(), "💯 love: ❤".to_owned());
    }

    #[test]
    fn test_string() {
        assert_eq!(String::from("a").param_name(), "a".to_owned());
        assert_eq!(String::from("+").param_name(), "+".to_owned());
        assert_eq!(String::from("é").param_name(), "é".to_owned());
        // This is the letter 'e' followed by a acute accent
        assert_eq!(String::from("é").param_name(), "e\u{301}".to_owned());
        assert_eq!(
            String::from("\u{0065}\u{0301}").param_name(),
            "e\u{301}".to_owned()
        );
        assert_eq!(
            String::from("\u{0065}\u{0301}").param_name(),
            "é".to_owned()
        );
        assert_eq!(
            String::from("💯 love: ❤").param_name(),
            "💯 love: ❤".to_owned()
        );
    }

    #[test]
    fn test_box() {
        let b = Box::new("A text");
        assert_eq!(b.param_name(), "A text".to_owned());
        let b = Box::new(42585u32);
        assert_eq!(b.param_name(), "42585".to_owned());
    }

    #[test]
    fn test_option() {
        assert_eq!(None::<u32>.param_name(), "None".to_owned());
        assert_eq!(Some(42585u32).param_name(), "42585".to_owned());
    }

    #[test]
    fn test_vec() {
        assert_eq!(Vec::<u32>::new().param_name(), "[]".to_owned());
        assert_eq!(vec![42585u32].param_name(), "[42585]".to_owned());
        assert_eq!(vec![4, 5].param_name(), "[4,5]".to_owned());
    }

    #[test]
    fn test_mutex() {
        assert_eq!(Mutex::new("a text").param_name(), "a text".to_owned());
    }

    #[test]
    fn test_tuple() {
        assert_eq!((5, 6).param_name(), "(5,6)".to_owned());
        assert_eq!(
            (5, false, "A text").param_name(),
            "(5,false,A text)".to_owned()
        );
        assert_eq!(
            (5, false, (Box::new(42), vec![5; 3])).param_name(),
            "(5,false,(42,[5,5,5]))".to_owned()
        );
    }
}
