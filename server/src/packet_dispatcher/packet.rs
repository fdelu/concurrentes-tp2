use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::dist_mutex::packets::{MutexPacket, Timestamp};
use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::packets::TwoPhaseCommitPacket;
use crate::two_phase_commit::ClientData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncRequestPacket {
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponsePacket {
    // TODO: use clients info
    pub database: HashMap<ClientId, ClientData>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Packet {
    Mutex(MutexPacket),
    Commit(TwoPhaseCommitPacket),
    SyncRequest(SyncRequestPacket),
    SyncResponse(SyncResponsePacket),
}
