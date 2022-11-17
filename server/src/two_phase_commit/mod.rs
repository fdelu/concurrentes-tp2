use crate::dist_mutex::packets::{get_timestamp, Timestamp};
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::Packet;
use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::packets::{
    CommitPacket, PreparePacket, RollbackPacket, Transaction, TwoPhaseCommitPacket, VoteNoPacket,
    VoteYesPacket,
};
use crate::ServerId;
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use tokio::sync::oneshot;

use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use common::AHandler;

pub mod messages;
pub mod packets;

pub type TransactionId = u32;

pub enum TransactionState {
    Prepared,
    Commit,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientData {
    pub points: u32,
    pub blocked_points: Vec<(TransactionId, Timestamp, u32)>,
}

const INITIAL_DATABASE: [(ClientId, ClientData); 5] = [
    (
        1u32,
        ClientData {
            points: 100,
            blocked_points: vec![],
        },
    ),
    (
        2u32,
        ClientData {
            points: 100,
            blocked_points: vec![],
        },
    ),
    (
        3u32,
        ClientData {
            points: 100,
            blocked_points: vec![],
        },
    ),
    (
        4u32,
        ClientData {
            points: 100,
            blocked_points: vec![],
        },
    ),
    (
        5u32,
        ClientData {
            points: 100,
            blocked_points: vec![],
        },
    ),
];

pub struct TwoPhaseCommit<P: Actor> {
    logs: HashMap<TransactionId, TransactionState>,
    stakeholder_timeouts: HashMap<TransactionId, oneshot::Sender<()>>,
    coordinator_timeouts: HashMap<TransactionId, oneshot::Sender<bool>>,
    confirmations: HashMap<TransactionId, HashSet<ServerId>>,
    dispatcher: Addr<P>,
    database: HashMap<ClientId, ClientData>,
    transactions: HashMap<TransactionId, Transaction>,
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

impl<P: Actor> TwoPhaseCommit<P> {
    pub fn new(dispatcher: Addr<P>) -> Self {
        let logs = HashMap::new();

        Self {
            logs,
            stakeholder_timeouts: HashMap::new(),
            coordinator_timeouts: HashMap::new(),
            confirmations: HashMap::new(),
            dispatcher,
            database: INITIAL_DATABASE.into(),
            transactions: HashMap::new(),
        }
    }

    fn prepare_transaction(&mut self, id: TransactionId, transaction: Transaction) -> bool {
        match transaction {
            Transaction::Block {
                id: client_id,
                amount,
            } => {
                let client_data = self.database.get_mut(&client_id).unwrap();
                if client_data.points >= amount {
                    // Ok
                    self.transactions.insert(
                        id,
                        Transaction::Block {
                            id: client_id,
                            amount,
                        },
                    );
                    true
                } else {
                    // Error, VoteNo
                    false
                }
            }
            Transaction::Increase {
                id: client_id,
                amount,
            } => {
                self.transactions.insert(
                    id,
                    Transaction::Increase {
                        id: client_id,
                        amount,
                    },
                );
                true
            }
            Transaction::Discount => {
                match self.transactions.get(&id) {
                    Some(Transaction::Block {
                        id: client_id,
                        amount,
                    }) => {
                        let client_data = self.database.get_mut(client_id).unwrap();
                        if client_data.points >= *amount {
                            // Ok
                            self.transactions.insert(
                                id,
                                Transaction::Block {
                                    id: *client_id,
                                    amount: *amount,
                                },
                            );
                            true
                        } else {
                            // This should never happen because I check that client has enough points before
                            // blocking them
                            panic!("Client {} doesn't have enough points", client_id);
                        }
                    }
                    _ => panic!("Invalid transaction"),
                }
            }
        }
    }

    fn commit_transaction(&mut self, id: TransactionId) {
        match self.transactions.get(&id) {
            Some(Transaction::Block {
                id: client_id,
                amount,
            }) => {
                let client_data = self.database.get_mut(client_id).unwrap();
                if client_data.points >= *amount {
                    client_data.points -= *amount;
                    client_data
                        .blocked_points
                        .push((id, get_timestamp(), *amount));
                } else {
                    // This should never happen because I check that client has enough points before
                    // voting yes
                    panic!("Client {} doesn't have enough points", client_id);
                }
            }
            Some(Transaction::Increase {
                id: client_id,
                amount,
            }) => {
                let client_data = self.database.get_mut(client_id).unwrap();
                client_data.points += amount;
            }
            Some(Transaction::Discount) => {
                match self.transactions.get(&id) {
                    Some(Transaction::Block {
                        id: client_id,
                        amount: _,
                    }) => {
                        let client_data = self.database.get_mut(client_id).unwrap();
                        client_data
                            .blocked_points
                            .retain(|(transaction_id, _, _)| *transaction_id != id);
                    }
                    _ => panic!("Invalid transaction"),
                }
                self.transactions.remove(&id);
            }
            _ => panic!("Invalid transaction"),
        }
    }

    fn abort_transaction(&mut self, id: TransactionId) {
        if let Some(Transaction::Block {
            id: client_id,
            amount,
        }) = self.transactions.remove(&id)
        {
            let client_data = self.database.get_mut(&client_id).unwrap();
            client_data
                .blocked_points
                .retain(|(transaction_id, _, _)| *transaction_id != id);
            client_data.points += amount;
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
