use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::time::Duration;

use actix::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use common::AHandler;

use crate::dist_mutex::packets::{get_timestamp, Timestamp};
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::Packet;
use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::packets::{
    CommitPacket, PreparePacket, RollbackPacket, Transaction, TwoPhaseCommitPacket, VoteNoPacket,
    VoteYesPacket,
};
use crate::ServerId;

pub mod messages;
pub mod packets;

const MAX_POINT_BLOCKING_TIME: Duration = Duration::from_secs(30);

pub type TransactionId = u32;

pub enum TransactionState {
    Prepared,
    Commit,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientData {
    pub points: u32,
    pub blocked_points: HashMap<TransactionId, u32>,
}

pub fn make_initial_database() -> HashMap<ClientId, ClientData> {
    let mut database = HashMap::new();
    for client_id in 0..10 {
        database.insert(
            client_id,
            ClientData {
                points: 100,
                blocked_points: HashMap::new(),
            },
        );
    }
    database
}

pub struct TwoPhaseCommit<P: Actor> {
    logs: HashMap<TransactionId, TransactionState>,
    stakeholder_timeouts: HashMap<TransactionId, oneshot::Sender<()>>,
    coordinator_timeouts: HashMap<TransactionId, oneshot::Sender<bool>>,
    blocked_points_timeouts: HashMap<TransactionId, SpawnHandle>,
    confirmations: HashMap<TransactionId, HashSet<ServerId>>,
    dispatcher: Addr<P>,
    database: HashMap<ClientId, ClientData>,
    database_last_update: Timestamp,
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
            blocked_points_timeouts: HashMap::new(),
            confirmations: HashMap::new(),
            dispatcher,
            database: make_initial_database(),
            database_last_update: get_timestamp(),
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
                    println!("Voting yes for transaction {} because client {} has enough points to block", id, client_id);
                    true
                } else {
                    println!(
                        "Voting no for transaction {} because client {} has not enough points",
                        id, client_id
                    );
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

    fn set_timeout_for_blocked_points(
        &mut self,
        id: TransactionId,
        of: ClientId,
        ctx: &mut Context<Self>,
    ) {
        let handle = ctx.run_later(MAX_POINT_BLOCKING_TIME, move |me, _| {
            println!(
                "{} Timeout while waiting for discount points of transaction {}, unblocking them",
                me, id
            );
            me.database.get_mut(&of).unwrap().blocked_points.remove(&id);
        });
        self.blocked_points_timeouts.insert(id, handle);
    }

    fn commit_transaction(&mut self, id: TransactionId, ctx: &mut Context<Self>) {
        self.database_last_update = get_timestamp();
        match self.transactions.get(&id) {
            Some(Transaction::Block {
                id: client_id,
                amount,
            }) => {
                let client_data = self.database.get_mut(client_id).unwrap();
                if client_data.points >= *amount {
                    client_data.points -= *amount;
                    client_data.blocked_points.insert(id, *amount);
                    self.set_timeout_for_blocked_points(id, *client_id, ctx);
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
                        client_data.blocked_points.remove(&id);
                        let h = self.blocked_points_timeouts.remove(&id).unwrap();
                        ctx.cancel_future(h);
                    }
                    _ => panic!("Invalid transaction"),
                }
                self.transactions.remove(&id);
            }
            _ => panic!("Invalid transaction"),
        }
        println!("Database: {:#?}", self.database);
    }

    fn abort_transaction(&mut self, id: TransactionId, ctx: &mut Context<Self>) {
        if let Some(Transaction::Block {
            id: client_id,
            amount,
        }) = self.transactions.remove(&id)
        {
            let client_data = self.database.get_mut(&client_id).unwrap();
            client_data.blocked_points.remove(&id);
            client_data.points += amount;
            if let Some(h) = self.blocked_points_timeouts.remove(&id) {
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
