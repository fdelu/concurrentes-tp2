use std::collections::{HashMap, HashSet};
use actix::prelude::*;
use common::packet::UserId;
use crate::dist_mutex::packets::Timestamp;
use crate::packet_dispatcher::TransactionId;
use crate::ServerId;
use crate::two_phase_commit::{CommitResult, TransactionState, UserData};
use crate::two_phase_commit::packets::Transaction;

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct VoteYesMessage {
    pub from: ServerId,
    pub id: TransactionId,
    pub connected_servers: HashSet<ServerId>,
}

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct VoteNoMessage {
    pub from: ServerId,
    pub id: TransactionId,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateDatabaseMessage {
    pub snapshot_from: Timestamp,
    pub database: HashMap<UserId, UserData>,
    pub logs: HashMap<TransactionId, (TransactionState, Transaction)>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RollbackMessage {
    pub id: TransactionId,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RemoveTransactionMessage {
    pub transaction_id: TransactionId,
}

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct PrepareMessage {
    pub from: ServerId,
    pub id: TransactionId,
    pub transaction: Transaction,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ForwardDatabaseMessage {
    pub to: ServerId,
}

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct CommitMessage {
    pub from: ServerId,
    pub id: TransactionId,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CommitCompleteMessage {
    pub id: TransactionId,
    pub connected_servers: HashSet<ServerId>,
}

#[derive(Message)]
#[rtype(result = "CommitResult<bool>")]
pub struct CommitRequestMessage {
    pub id: TransactionId,
    pub transaction: Transaction,
}
