use serde::{Deserialize, Serialize};

use crate::dist_mutex::packets::{MutexPacket, Timestamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncRequestPacket {
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponsePacket {
    // TODO: use clients info
    pub data: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Packet {
    Mutex(MutexPacket),
    // To be implemented
    Commit,
    SyncRequest(SyncRequestPacket),
    SyncResponse(SyncResponsePacket),
}