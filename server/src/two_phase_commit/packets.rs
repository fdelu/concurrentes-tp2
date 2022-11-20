use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::TransactionId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TwoPhaseCommitPacket {
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
    Discount { id: ClientId, amount: u32 },
    Increase { id: ClientId, amount: u32 },
}
