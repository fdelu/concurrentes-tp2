use actix::prelude::*;
use tracing::warn;

use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::two_phase_commit::{CommitResult, TransactionId, TransactionState, TwoPhaseCommit};
use crate::ServerId;
use common::AHandler;

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct VoteNoMessage {
    pub from: ServerId,
    pub id: TransactionId,
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
