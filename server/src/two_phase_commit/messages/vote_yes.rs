use std::collections::HashSet;

use actix::prelude::*;

use common::AHandler;

use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::two_phase_commit::{CommitResult, TransactionId, TwoPhaseCommit};
use crate::ServerId;

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct VoteYesMessage {
    pub from: ServerId,
    pub id: TransactionId,
    pub connected_servers: HashSet<ServerId>,
}

impl<P: AHandler<BroadcastMessage>> Handler<VoteYesMessage> for TwoPhaseCommit<P> {
    type Result = CommitResult<()>;

    fn handle(&mut self, msg: VoteYesMessage, _ctx: &mut Self::Context) -> Self::Result {
        println!(
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
