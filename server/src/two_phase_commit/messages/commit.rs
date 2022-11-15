use actix::prelude::*;
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::ServerId;
use crate::two_phase_commit::{CommitResult, TransactionId, TransactionState, TwoPhaseCommit};

use common::AHandler;

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct CommitMessage {
    pub from: ServerId,
    pub id: TransactionId,
}

impl<P: AHandler<SendMessage>> Handler<CommitMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<()>>;

    fn handle(&mut self, msg: CommitMessage, _ctx: &mut Self::Context) -> Self::Result {
        println!("{} Received commit from {} for {}", self, msg.from, msg.id);
        self.timeout_channels.remove(&msg.id).map(|tx| tx.send(()));

        match self.logs.get(&msg.id) {
            Some(TransactionState::Prepared) => {
                // TODO: Do changes internally
                self.logs.insert(msg.id, TransactionState::Commit);
                // TODO: Send Ack
            },
            Some(TransactionState::Commit) => {
                // TODO: Send Ack
            },
            _ => {
                // TODO: Fail
            }
        }
        async move {
            Ok(())
        }.into_actor(self).boxed_local()
    }
}