use crate::packet_dispatcher::messages::send::SendMessage;
use crate::two_phase_commit::{CommitResult, TransactionId, TransactionState, TwoPhaseCommit};
use crate::ServerId;
use actix::prelude::*;

use common::AHandler;

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct CommitMessage {
    pub from: ServerId,
    pub id: TransactionId,
}

impl<P: AHandler<SendMessage>> Handler<CommitMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<()>>;

    fn handle(&mut self, msg: CommitMessage, ctx: &mut Self::Context) -> Self::Result {
        println!("{} Received commit from {} for {}", self, msg.from, msg.id);
        self.stakeholder_timeouts
            .remove(&msg.id)
            .map(|tx| tx.send(()));

        match self.logs.get(&msg.id) {
            Some(TransactionState::Prepared) => {
                self.logs.insert(msg.id, TransactionState::Commit);
                self.commit_transaction(msg.id, ctx);
            }
            Some(TransactionState::Commit) => {
                // TODO: Send Ack
            }
            _ => {
                // TODO: Fail
                // It is possible that a transaction was being held while syncing the server
                // and there is no entry in the logs
                // In this case, restart the server and send a new sync request
            }
        }
        async move { Ok(()) }.into_actor(self).boxed_local()
    }
}
