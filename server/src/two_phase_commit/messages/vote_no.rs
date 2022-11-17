use actix::prelude::*;

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

    fn handle(&mut self, msg: VoteNoMessage, _ctx: &mut Self::Context) -> Self::Result {
        println!("{} Received vote no from {} for {}", self, msg.from, msg.id);
        self.stakeholder_timeouts
            .remove(&msg.id)
            .map(|tx| tx.send(()));

        self.logs.insert(msg.id, TransactionState::Abort);
        self.abort_transaction(msg.id);

        self.broadcast_rollback(msg.id);
        Ok(())
    }
}
