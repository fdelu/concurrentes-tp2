use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use actix::prelude::*;
use tokio::sync::oneshot;

mod packets;
mod messages;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TransactionId {
    id: u32,
}

pub enum TransactionState {
    Prepared,
    Commit,
    Abort,
}

impl From<TransactionId> for [u8; 4] {
    fn from(transaction_id: TransactionId) -> Self {
        transaction_id.id.to_be_bytes()
    }
}

impl From<&[u8]> for TransactionId {
    fn from(bytes: &[u8]) -> Self {
        let id = u32::from_be_bytes(bytes.try_into().unwrap());
        Self { id }
    }
}

pub struct TwoPhaseCommit<P: Actor> {
    logs: HashMap<TransactionId, TransactionState>,
    timeout_channels: HashMap<TransactionId, oneshot::Sender<()>>,
    dispatcher: Addr<P>,
}

impl<P: Actor> Actor for TwoPhaseCommit<P> {
    type Context = Context<Self>;
}

impl<P: Actor> Display for TwoPhaseCommit<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[TwoPhaseCommit]")
    }
}

impl Display for TransactionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[TransactionId {}]", self.id)
    }
}

pub enum CommitError {
    Timeout,
    Disconnected,
}

pub type CommitResult<T> = Result<T, CommitError>;