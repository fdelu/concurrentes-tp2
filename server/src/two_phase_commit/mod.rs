use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::time::Duration;

use actix::prelude::*;
use common::error::CoffeeError;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use common::packet::UserId;
use common::AHandler;
use tracing::{debug, info, trace, warn};

use crate::dist_mutex::packets::{get_timestamp, Timestamp};
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::Packet;
use crate::packet_dispatcher::TransactionId;
use crate::two_phase_commit::messages::remove_transaction::RemoveTransactionMessage;
use crate::two_phase_commit::packets::{
    CommitPacket, PreparePacket, RollbackPacket, Transaction, TwoPhaseCommitPacket, VoteNoPacket,
    VoteYesPacket,
};
use crate::ServerId;

pub mod messages;
pub mod packets;

const MAX_POINT_BLOCKING_TIME: Duration = Duration::from_secs(30);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionState {
    Prepared,
    Commit,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
    pub points: u32,
}

pub fn make_initial_database() -> HashMap<UserId, UserData> {
    let mut database = HashMap::new();
    for client_id in 0..10 {
        database.insert(client_id, UserData { points: 100 });
    }
    database
}

pub struct TwoPhaseCommit<P: Actor> {
    logs: HashMap<TransactionId, (TransactionState, Transaction)>,
    coordinator_timeouts: HashMap<TransactionId, oneshot::Sender<bool>>,
    transactions_timeouts: HashMap<TransactionId, SpawnHandle>,
    confirmations: HashMap<TransactionId, HashSet<ServerId>>,
    dispatcher: Addr<P>,
    database: HashMap<UserId, UserData>,
    database_last_update: Timestamp,
}

impl<P: Actor> Actor for TwoPhaseCommit<P> {
    type Context = Context<Self>;
}

impl<P: Actor> Display for TwoPhaseCommit<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[TwoPhaseCommit]")
    }
}

#[derive(Debug)]
pub enum CommitError {
    Timeout,
    Disconnected,
}

pub type CommitResult<T> = Result<T, CommitError>;

impl<P: Actor> TwoPhaseCommit<P> {
    pub fn new(dispatcher: Addr<P>) -> Addr<Self> {
        let logs = HashMap::new();
        Self::create(|ctx| {
            ctx.run_interval(Duration::from_secs(10), |me, _| {
                trace!("Database: {:#?}", me.database);
            });
            Self {
                logs,
                coordinator_timeouts: HashMap::new(),
                transactions_timeouts: HashMap::new(),
                confirmations: HashMap::new(),
                dispatcher,
                database: make_initial_database(),
                database_last_update: 0,
            }
        })
    }

    fn prepare_transaction(
        &mut self,
        transaction_id: TransactionId,
        transaction: Transaction,
        ctx: &mut Context<Self>,
    ) -> bool {
        self.logs
            .insert(transaction_id, (TransactionState::Prepared, transaction));
        match transaction {
            Transaction::Discount {
                id: client_id,
                amount,
            } => {
                let client_data = self.get_or_create_user(&client_id);
                if client_data.points >= amount {
                    debug!(
                        "Client {} has enough points to block (needed: {}, actual: {})",
                        client_id, amount, client_data.points
                    );
                    client_data.points -= amount;
                    self.set_timeout_for_transaction(transaction_id, ctx);
                    true
                } else {
                    warn!(
                        "Client {} does not have enough points (needed: {}, actual: {})",
                        client_id, amount, client_data.points
                    );
                    self.logs
                        .insert(transaction_id, (TransactionState::Abort, transaction));
                    false
                }
            }
            Transaction::Increase {
                id: client_id,
                amount,
            } => {
                self.get_or_create_user(&client_id).points += amount;
                self.set_timeout_for_transaction(transaction_id, ctx);
                true
            }
        }
    }

    fn get_or_create_user(&mut self, client_id: &UserId) -> &mut UserData {
        if !self.database.contains_key(client_id) {
            self.database.insert(*client_id, UserData { points: 100 });
        }
        self.database.get_mut(client_id).unwrap()
    }

    fn set_timeout_for_transaction(&mut self, id: TransactionId, ctx: &mut Context<Self>) {
        if self.transactions_timeouts.get(&id).is_some() {
            return;
        }
        let handle = ctx.notify_later(
            RemoveTransactionMessage { transaction_id: id },
            MAX_POINT_BLOCKING_TIME,
        );
        self.transactions_timeouts.insert(id, handle);
    }

    fn commit_transaction(&mut self, id: TransactionId, ctx: &mut Context<Self>) {
        info!("Committing transaction {}", id);
        if let Some(h) = self.transactions_timeouts.remove(&id) {
            ctx.cancel_future(h);
        };
        self.database_last_update = get_timestamp();
        self.logs.get_mut(&id).unwrap().0 = TransactionState::Commit;
    }

    fn abort_transaction(&mut self, id: TransactionId, ctx: &mut Context<Self>) {
        self.transactions_timeouts.remove(&id);
        if let Some((state, transaction)) = self.logs.remove(&id) {
            if state != TransactionState::Abort {
                self.logs.insert(id, (TransactionState::Abort, transaction));
                match transaction {
                    Transaction::Discount {
                        id: client_id,
                        amount,
                    } => {
                        let client_data = self.get_or_create_user(&client_id);
                        client_data.points += amount;
                    }
                    Transaction::Increase {
                        id: client_id,
                        amount,
                    } => {
                        let client_data = self.get_or_create_user(&client_id);
                        client_data.points -= amount;
                    }
                }
            }
            if let Some(h) = self.transactions_timeouts.remove(&id) {
                ctx.cancel_future(h);
            }
        }
    }
}

impl<P: AHandler<SendMessage>> TwoPhaseCommit<P> {
    fn send_vote_yes(&mut self, to: ServerId, id: TransactionId) {
        self.dispatcher.do_send(SendMessage {
            to,
            packet: Packet::Commit(TwoPhaseCommitPacket::VoteYes(VoteYesPacket { id })),
        });
    }

    fn send_vote_no(&mut self, to: ServerId, id: TransactionId) {
        self.dispatcher.do_send(SendMessage {
            to,
            packet: Packet::Commit(TwoPhaseCommitPacket::VoteNo(VoteNoPacket { id })),
        });
    }
}

impl<P: AHandler<BroadcastMessage>> TwoPhaseCommit<P> {
    fn broadcast_rollback(&mut self, id: TransactionId) {
        self.dispatcher.do_send(BroadcastMessage {
            packet: Packet::Commit(TwoPhaseCommitPacket::Rollback(RollbackPacket { id })),
        });
    }

    fn broadcast_commit(&mut self, id: TransactionId) {
        self.dispatcher.do_send(BroadcastMessage {
            packet: Packet::Commit(TwoPhaseCommitPacket::Commit(CommitPacket { id })),
        });
    }

    fn broadcast_prepare(&mut self, packet: PreparePacket) {
        self.dispatcher.do_send(BroadcastMessage {
            packet: Packet::Commit(TwoPhaseCommitPacket::Prepare(packet)),
        });
    }
}

#[derive(Debug)]
pub enum PacketDispatcherError {
    Timeout,
    InsufficientPoints,
    DiscountFailed,
    IncreaseFailed,
    Other,
}

pub type PacketDispatcherResult<T> = Result<T, PacketDispatcherError>;

impl Display for PacketDispatcherError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<PacketDispatcherError> for CoffeeError {
    fn from(e: PacketDispatcherError) -> Self {
        match e {
            PacketDispatcherError::InsufficientPoints => CoffeeError::InsufficientPoints,
            _ => Self::new(&e.to_string()),
        }
    }
}
