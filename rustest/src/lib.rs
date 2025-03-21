use core::any::{Any, TypeId};

pub use libtest_mimic;
pub use libtest_mimic::Failed;
pub use rustest_macro::{fixture, main, test};

pub type Result = std::result::Result<(), Failed>;

pub trait CollectError {
    fn collect_error(self) -> Result;
}

impl CollectError for () {
    fn collect_error(self) -> Result {
        Ok(())
    }
}

impl<T> CollectError for std::result::Result<(), T>
where
    T: Into<Failed>,
{
    fn collect_error(self) -> Result {
        self.map_err(|e| e.into())
    }
}

pub trait Fixture: Clone {
    fn setup(ctx: &mut Context) -> Self
    where
        Self: Sized;
}

pub struct Context {
    fixtures: std::collections::HashMap<TypeId, Option<Box<dyn Any>>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            fixtures: Default::default(),
        }
    }

    pub fn register_fixture(&mut self, id: TypeId) {
        self.fixtures.insert(id, None);
    }

    pub fn get_fixture<T>(&mut self) -> T
    where
        T: Fixture + Any,
    {
        if !self.fixtures.contains_key(&TypeId::of::<T>()) {
            return T::setup(self);
        }

        if let Some(f) = self.fixtures.get(&TypeId::of::<T>()).unwrap() {
            let fixture = f.downcast_ref::<T>().unwrap();
            return fixture.clone();
        }

        let value = T::setup(self);
        self.fixtures
            .insert(TypeId::of::<T>(), Some(Box::new(value.clone())));
        value
    }
}
