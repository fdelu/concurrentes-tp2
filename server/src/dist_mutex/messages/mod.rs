use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) mod ack;
pub(crate) mod ok;
pub mod public;
pub(crate) mod request;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct Timestamp {
    time: u128,
}

impl Timestamp {
    pub fn new() -> Self {
        Self {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        }
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Timestamp> for [u8; 16] {
    fn from(timestamp: Timestamp) -> Self {
        timestamp.time.to_be_bytes()
    }
}

impl From<&[u8]> for Timestamp {
    fn from(bytes: &[u8]) -> Self {
        let time = u128::from_be_bytes(bytes.try_into().unwrap());
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
