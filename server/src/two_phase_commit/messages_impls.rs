use std::collections::HashSet;
use actix::prelude::*;
use tracing::{debug, error, info, warn};
use common::AHandler;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time;
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::messages::public::die::DieMessage;
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::{Packet, SyncResponsePacket};
use crate::two_phase_commit::{CommitError, CommitResult, TransactionState, TwoPhaseCommit};
use crate::two_phase_commit::messages::{CommitCompleteMessage, CommitMessage, CommitRequestMessage, ForwardDatabaseMessage, PrepareMessage, RemoveTransactionMessage, RollbackMessage, UpdateDatabaseMessage, VoteNoMessage, VoteYesMessage};
use crate::two_phase_commit::packets::PreparePacket;

const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_millis(5000);

impl<P: AHandler<BroadcastMessage>> Handler<VoteYesMessage> for TwoPhaseCommit<P> {
    type Result = CommitResult<()>;

    fn handle(&mut self, msg: VoteYesMessage, _ctx: &mut Self::Context) -> Self::Result {
        debug!(
            "{} Received vote yes from {} for {}",
            self, msg.from, msg.id
        );

        let confirmed_servers = self
            .confirmations
            .entry(msg.id)
            .or_insert_with(HashSet::new);
        confirmed_servers.insert(msg.from);
        if msg.connected_servers.is_superset(confirmed_servers) {
            self.coordinator_timeouts
                .remove(&msg.id)
                .map(|tx| tx.send(true));
        }
        Ok(())
    }
}

impl<P: AHandler<BroadcastMessage>> Handler<VoteNoMessage> for TwoPhaseCommit<P> {
    type Result = CommitResult<()>;

    fn handle(&mut self, msg: VoteNoMessage, ctx: &mut Self::Context) -> Self::Result {
        warn!("{} Received vote no from {} for {}", self, msg.from, msg.id);
        self.coordinator_timeouts
            .remove(&msg.id)
            .map(|tx| tx.send(false));

        if let Some((state, _)) = self.logs.get_mut(&msg.id) {
            *state = TransactionState::Abort;
        }
        self.abort_transaction(msg.id, ctx);
        self.broadcast_rollback(msg.id);
        Ok(())
    }
}

impl<P: Actor> Handler<UpdateDatabaseMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: UpdateDatabaseMessage, ctx: &mut Self::Context) -> Self::Result {
        if msg.snapshot_from > self.database_last_update {
            debug!(
                "Updating database from {} to {}",
                self.database_last_update, msg.snapshot_from
            );
            msg.logs.iter().for_each(|(id, (state, _))| {
                if *state == TransactionState::Prepared {
                    self.set_timeout_for_transaction(*id, ctx);
                }
            });
            self.database = msg.database;
            self.logs = msg.logs;
            self.database_last_update = msg.snapshot_from;
        } else {
            debug!(
                "Ignoring database update ({} >= {})",
                self.database_last_update, msg.snapshot_from
            );
        }
    }
}

impl<P: Actor> Handler<RollbackMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: RollbackMessage, ctx: &mut Self::Context) -> Self::Result {
        info!("{} Received rollback for {}", self, msg.id);

        if let Some((state, _)) = self.logs.get_mut(&msg.id) {
            *state = TransactionState::Abort;
        }
        self.abort_transaction(msg.id, ctx);
    }
}

impl<P: Actor> Handler<RemoveTransactionMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: RemoveTransactionMessage, ctx: &mut Self::Context) -> Self::Result {
        error!(
            "{} Timeout while waiting for transaction {}, aborting it",
            self, msg.transaction_id
        );
        self.abort_transaction(msg.transaction_id, ctx);
    }
}

impl<P: AHandler<SendMessage>> Handler<PrepareMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<()>>;

    fn handle(&mut self, msg: PrepareMessage, ctx: &mut Self::Context) -> Self::Result {
        debug!("{} Received prepare from {}", self, msg.from);

        match self.logs.get(&msg.id) {
            Some((TransactionState::Prepared | TransactionState::Commit, _)) => {
                self.send_vote_yes(msg.from, msg.id);
            }
            Some((TransactionState::Abort, _)) => {
                self.send_vote_no(msg.from, msg.id);
            }
            None => {
                debug!("{} Doing transaction", self);
                if self.prepare_transaction(msg.id, msg.transaction, ctx) {
                    self.send_vote_yes(msg.from, msg.id);
                } else {
                    self.send_vote_no(msg.from, msg.id);
                }
            }
        };
        async { Ok(()) }.into_actor(self).boxed_local()
    }
}

impl<P: AHandler<SendMessage>> Handler<ForwardDatabaseMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: ForwardDatabaseMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.dispatcher.do_send(SendMessage {
            to: msg.to,
            packet: Packet::SyncResponse(SyncResponsePacket {
                snapshot_from: self.database_last_update,
                database: self.database.clone(),
                logs: self.logs.clone(),
            }),
        });
    }
}

impl<P: AHandler<SendMessage> + AHandler<DieMessage>> Handler<CommitMessage> for TwoPhaseCommit<P> {
    type Result = CommitResult<()>;

    fn handle(&mut self, msg: CommitMessage, ctx: &mut Self::Context) -> Self::Result {
        debug!("{} Received commit from {} for {}", self, msg.from, msg.id);

        if let Some((state, _)) = self.logs.get_mut(&msg.id) {
            if *state == TransactionState::Prepared {
                *state = TransactionState::Commit;
                self.commit_transaction(msg.id, ctx);
            }
        } else {
            self.dispatcher.do_send(DieMessage);
        }
        Ok(())
    }
}

impl<P: AHandler<BroadcastMessage>> Handler<CommitCompleteMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: CommitCompleteMessage, ctx: &mut Self::Context) -> Self::Result {
        debug!("{} Trying to commit {}", self, msg.id);
        let confirmed_servers = self
            .confirmations
            .entry(msg.id)
            .or_insert_with(HashSet::new);

        if confirmed_servers.is_superset(&msg.connected_servers) {
            if let Some((state, _)) = self.logs.get_mut(&msg.id) {
                if *state == TransactionState::Abort {
                    return;
                }
                *state = TransactionState::Commit;
                self.commit_transaction(msg.id, ctx);
                self.broadcast_commit(msg.id);
            }
        } else {
            debug!(
                "{} Not committing {} because not all servers have confirmed",
                self, msg.id
            );
        }
    }
}

impl<P: AHandler<BroadcastMessage>> Handler<CommitRequestMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<bool>>;

    fn handle(&mut self, msg: CommitRequestMessage, ctx: &mut Self::Context) -> Self::Result {
        let prepare_packet = PreparePacket::new(msg.id, msg.transaction);
        let id = prepare_packet.id;
        if !self.prepare_transaction(id, prepare_packet.transaction, ctx) {
            return Box::pin(async { Ok(false) }.into_actor(self));
        }

        self.broadcast_prepare(prepare_packet);

        let (tx, rx) = oneshot::channel();
        self.coordinator_timeouts.insert(id, tx);

        async move {
            let r = time::timeout(TIME_UNTIL_DISCONNECT_POLITIC, rx).await;
            r.map_err(|_| CommitError::Timeout).map(|r| r.unwrap())
        }
        .into_actor(self)
        .boxed_local()
    }
}
