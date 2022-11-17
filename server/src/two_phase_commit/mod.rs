use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use actix::prelude::*;
use tokio::sync::oneshot;

mod packets;
mod messages;

pub type TransactionId = u32;

pub enum TransactionState {
    Prepared,
    Commit,
    Abort,
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


pub enum CommitError {
    Timeout,
    Disconnected,
}

pub type CommitResult<T> = Result<T, CommitError>;