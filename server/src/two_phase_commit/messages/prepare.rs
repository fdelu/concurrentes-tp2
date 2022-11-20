use actix::prelude::*;

use common::AHandler;
use tracing::debug;

use crate::packet_dispatcher::messages::send::SendMessage;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{CommitResult, TransactionId, TransactionState, TwoPhaseCommit};
use crate::ServerId;

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct PrepareMessage {
    pub from: ServerId,
    pub id: TransactionId,
    pub transaction: Transaction,
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
