use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::two_phase_commit::{TransactionId, TransactionState, TwoPhaseCommit};
use actix::prelude::*;
use std::collections::HashSet;
use tracing::debug;

use crate::ServerId;
use common::AHandler;

#[derive(Message)]
#[rtype(result = "()")]
pub struct CommitCompleteMessage {
    pub id: TransactionId,
    pub connected_servers: HashSet<ServerId>,
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
