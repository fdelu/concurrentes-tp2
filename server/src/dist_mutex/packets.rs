use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

pub type Timestamp = u128;
pub type ResourceId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutexPacket {
    Request(RequestPacket),
    Ok(OkPacket),
    Ack(AckPacket),
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct AckPacket {
    pub id: ResourceId,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct OkPacket {
    pub id: ResourceId,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct RequestPacket {
    pub id: ResourceId,
    pub timestamp: Timestamp,
}

impl RequestPacket {
    pub fn new(id: ResourceId) -> Self {
        Self {
            id,
            timestamp: get_timestamp()
        }
    }
}

pub fn get_timestamp() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
