use std::collections::HashMap;
use serde_with::serde_as;
use common::packet::UserId;
use serde::{Deserialize, Serialize};

use crate::dist_mutex::packets::{MutexPacket, Timestamp};
use crate::packet_dispatcher::TransactionId;
use crate::two_phase_commit::packets::{Transaction, TwoPhaseCommitPacket};
use crate::two_phase_commit::{TransactionState, UserData};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncRequestPacket {
    pub timestamp: Timestamp,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponsePacket {
    pub snapshot_from: Timestamp,
    pub database: HashMap<UserId, UserData>,
    #[serde_as(as = "Vec<(_, _)>")]
    pub logs: HashMap<TransactionId, (TransactionState, Transaction)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Packet {
    Mutex(MutexPacket),
    Commit(TwoPhaseCommitPacket),
    SyncRequest(SyncRequestPacket),
    SyncResponse(SyncResponsePacket),
}
