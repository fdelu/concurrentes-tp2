use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

mod public;
mod ack;
mod ok;
mod request;

#[derive(Eq, PartialEq)]
pub struct Timestamp {
    time: u128,
}

impl Timestamp {
    pub fn new() -> Self {
        Self {
            time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
        }
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Timestamp> for u128 {
    fn from(timestamp: Timestamp) -> Self {
        timestamp.time
    }
}

impl From<u128> for Timestamp {
    fn from(time: u128) -> Self {
        Self { time }
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}