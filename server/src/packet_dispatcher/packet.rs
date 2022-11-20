use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dist_mutex::packets::{MutexPacket, Timestamp};
use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::packets::{Transaction, TwoPhaseCommitPacket};
use crate::two_phase_commit::{ClientData, TransactionId, TransactionState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncRequestPacket {
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponsePacket {
    pub snapshot_from: Timestamp,
    pub database: HashMap<ClientId, ClientData>,
    pub logs: HashMap<TransactionId, (TransactionState, Transaction)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Packet {
    Mutex(MutexPacket),
    Commit(TwoPhaseCommitPacket),
    SyncRequest(SyncRequestPacket),
    SyncResponse(SyncResponsePacket),
}
