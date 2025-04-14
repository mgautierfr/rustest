//! Example showing how to write sequentially executed tests.
//! `sleep` is used to introduce artificial delay and the unix timestamp is printed to show that
//! they indeed run sequentially with no overlap in execution time.

use std::thread::sleep;
use std::time::Duration;

pub struct Client {}

impl Client {
    pub fn new() -> Self {
        println!("Hello, world!");
        Self {}
    }
    pub fn add(&self, left: u64, right: u64) -> u64 {
        sleep(Duration::from_millis(100));
        left + right
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        println!("goodbye!");
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use googletest::prelude::*;
    use rustest::{FixtureDisplay, fixture, test};
    use std::ops::Deref;
    use std::sync::Mutex;
    use std::thread::sleep;
    use std::time::{Duration, UNIX_EPOCH};

    struct ClientWrapper(pub super::Client);

    impl Deref for ClientWrapper {
        type Target = super::Client;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[fixture(scope=global)]
    fn Client() -> Mutex<ClientWrapper> {
        Mutex::new(ClientWrapper(super::Client::new()))
    }

    impl FixtureDisplay for ClientWrapper {
        fn display(&self) -> String {
            "client".to_string()
        }
    }

    #[test]
    fn test1(client: Client) {
        let client = client.lock().unwrap();
        println!(
            "1: {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        let result = client.add(2, 2);
        println!(
            "1: {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        sleep(Duration::from_millis(200));
        println!(
            "1: {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        assert_that!(result, eq(4));
    }

    #[test]
    fn test2(client: Client) {
        let client = client.lock().unwrap();
        println!(
            "2: {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        let result = client.add(2, 2);
        println!(
            "2: {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        sleep(Duration::from_millis(200));
        println!(
            "2: {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        assert_that!(result, eq(4));
    }
}
