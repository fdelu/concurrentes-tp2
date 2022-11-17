use std::time::Duration;

use actix::prelude::*;
use tokio::sync::oneshot;
use tokio::time;

use common::AHandler;

use crate::packet_dispatcher::messages::send::SendMessage;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{
    CommitError, CommitResult, TransactionId, TransactionState, TwoPhaseCommit,
};
use crate::ServerId;

const SLEEP_TIME: Duration = Duration::from_millis(5000);

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct PrepareMessage {
    pub from: ServerId,
    pub id: TransactionId,
    pub transaction: Transaction,
}

impl<P: AHandler<SendMessage>> Handler<PrepareMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<()>>;

    fn handle(&mut self, msg: PrepareMessage, _ctx: &mut Self::Context) -> Self::Result {
        println!("{} Received prepare from {}", self, msg.from);

        match self.logs.get(&msg.id) {
            Some(TransactionState::Prepared | TransactionState::Commit) => {
                self.send_vote_yes(msg.from, msg.id);
            }
            Some(TransactionState::Abort) => {
                self.send_vote_no(msg.from, msg.id);
            }
            None => {
                println!("{} Doing transaction", self);
                self.logs.insert(msg.id, TransactionState::Prepared);
                if self.prepare_transaction(msg.id, msg.transaction) {
                    self.logs.insert(msg.id, TransactionState::Prepared);
                    self.send_vote_yes(msg.from, msg.id);
                } else {
                    self.logs.insert(msg.id, TransactionState::Abort);
                    self.send_vote_no(msg.from, msg.id);
                }
            }
        };
        let (tx, rx) = oneshot::channel();
        self.stakeholder_timeouts.insert(msg.id, tx);
        let transaction_id = msg.id;

        async move {
            if time::timeout(SLEEP_TIME, rx).await.is_err() {
                println!("[TwoPhaseCommit] Timeout");
                Err(CommitError::Timeout)
            } else {
                println!("[TwoPhaseCommit] Received commit");
                Ok(())
            }
        }
        .into_actor(self)
        .map(move |r, me, _| {
            if r.is_err() {
                if let Some(TransactionState::Prepared) = me.logs.get(&transaction_id) {
                    println!(
                        "{} Timeout while waiting for commit for {}",
                        me, transaction_id
                    );
                    me.logs.insert(transaction_id, TransactionState::Abort);
                    Err(CommitError::Timeout)
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        })
        .boxed_local()
    }
}
