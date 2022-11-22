use common::packet::UserId;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;

use crate::dist_mutex::packets::{MutexPacket, Timestamp};
use crate::packet_dispatcher::TransactionId;
use crate::two_phase_commit::messages::UpdateDatabaseMessage;
use crate::two_phase_commit::packets::{TPCommitPacket, Transaction};
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

/// Cada uno de los paquetes que se pueden enviar a través del [`PacketDispatcher`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Packet {
    Mutex(MutexPacket),
    Commit(TPCommitPacket),
    SyncRequest(SyncRequestPacket),
    SyncResponse(SyncResponsePacket),
}

impl SyncResponsePacket {
    /// Convierte el paquete en un mensaje [`UpdateDatabaseMessage`].
    pub fn to_update_db_msg(self) -> UpdateDatabaseMessage {
        UpdateDatabaseMessage {
            snapshot_from: self.snapshot_from,
            database: self.database,
            logs: self.logs,
        }
    }
}
