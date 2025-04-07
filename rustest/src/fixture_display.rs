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
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, bool, char, f32, f64, str, usize
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

impl FixtureDisplay for String {
    fn display(&self) -> String {
        self.clone()
    }
}

impl<T: FixtureDisplay> FixtureDisplay for Vec<T> {
    fn display(&self) -> String {
        self.iter()
            .map(|v| v.display())
            .collect::<Vec<_>>()
            .join(",")
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
