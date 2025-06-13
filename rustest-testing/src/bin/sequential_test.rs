//! Example showing how to write sequentially executed tests.
//! `sleep` is used to introduce artificial delay and the unix timestamp is printed to show that
//! they indeed run sequentially with no overlap in execution time.

use rustest::main;
use std::thread::sleep;
use std::time::Duration;

pub struct Client;

impl Client {
    pub fn new() -> Self {
        eprintln!("creating new client");
        Self {}
    }
    pub fn add(&self, left: u64, right: u64) -> u64 {
        sleep(Duration::from_millis(100));
        left + right
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        eprintln!("dropping client");
    }
}

pub mod tests {
    use googletest::prelude::*;
    use rustest::{fixture, test};
    use std::sync::Mutex;
    use std::thread::sleep;
    use std::time::{Duration, UNIX_EPOCH};

    #[fixture(scope=global)]
    fn Client() -> Mutex<super::Client> {
        Mutex::new(super::Client::new())
    }

    #[test]
    fn test1(client: Client) {
        let client = client.lock().unwrap();
        eprintln!(
            "1 {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        let result = client.add(2, 2);
        eprintln!(
            "1 {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        sleep(Duration::from_millis(200));
        eprintln!(
            "1 {}",
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
        eprintln!(
            "2 {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        let result = client.add(2, 2);
        eprintln!(
            "2 {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        sleep(Duration::from_millis(200));
        eprintln!(
            "2 {}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
        assert_that!(result, eq(4));
    }
}

#[main]
fn main() {}
