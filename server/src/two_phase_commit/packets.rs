use crate::two_phase_commit::messages::commit::CommitMessage;
use crate::two_phase_commit::messages::prepare::PrepareMessage;
use crate::two_phase_commit::messages::rollback::RollbackMessage;
use crate::two_phase_commit::messages::vote_no::VoteNoMessage;
use crate::two_phase_commit::messages::vote_yes::VoteYesMessage;
use crate::two_phase_commit::TransactionId;
use crate::ServerId;
use common::packet::UserId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TPCommitPacket {
    VoteYes(VoteYesPacket),
    VoteNo(VoteNoPacket),
    Commit(CommitPacket),
    Rollback(RollbackPacket),
    Prepare(PreparePacket),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreparePacket {
    pub id: TransactionId,
    pub transaction: Transaction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteYesPacket {
    pub id: TransactionId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteNoPacket {
    pub id: TransactionId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommitPacket {
    pub id: TransactionId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RollbackPacket {
    pub id: TransactionId,
}

impl PreparePacket {
    pub fn new(id: TransactionId, transaction: Transaction) -> Self {
        Self { id, transaction }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Transaction {
    Discount { id: UserId, amount: u32 },
    Increase { id: UserId, amount: u32 },
}

impl PreparePacket {
    pub fn to_message(self, from: ServerId) -> PrepareMessage {
        PrepareMessage {
            id: self.id,
            from,
            transaction: self.transaction,
        }
    }
}

impl VoteYesPacket {
    pub fn to_message(
        self,
        from: ServerId,
        connected_servers: HashSet<ServerId>,
    ) -> VoteYesMessage {
        VoteYesMessage {
            id: self.id,
            from,
            connected_servers,
        }
    }
}

impl VoteNoPacket {
    pub fn to_message(self, from: ServerId) -> VoteNoMessage {
        VoteNoMessage { id: self.id, from }
    }
}

impl CommitPacket {
    pub fn to_message(self, from: ServerId) -> CommitMessage {
        CommitMessage { id: self.id, from }
    }
}

impl RollbackPacket {
    pub fn to_message(self) -> RollbackMessage {
        RollbackMessage { id: self.id }
    }
}
