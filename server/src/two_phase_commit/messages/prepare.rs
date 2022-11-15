use std::time::Duration;
use actix::prelude::*;
use tokio::sync::oneshot;
use tokio::time;

use common::AHandler;

use crate::packet_dispatcher::messages::send::SendMessage;
use crate::ServerId;
use crate::two_phase_commit::{CommitError, CommitResult, TransactionId, TransactionState, TwoPhaseCommit};

const SLEEP_TIME: Duration = Duration::from_millis(5000);

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct PrepareMessage {
    pub from: ServerId,
    pub id: TransactionId,
}

impl<P: AHandler<SendMessage>> Handler<PrepareMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<()>>;

    fn handle(&mut self, msg: PrepareMessage, _ctx: &mut Self::Context) -> Self::Result {
        println!("{} Received prepare from {}", self, msg.from);
        match self.logs.get(&msg.id) {
            Some(TransactionState::Prepared | TransactionState::Commit) => {
                println!("{} Sending commit to {}", self, msg.from);
                // TODO: Send commit
            }
            Some(TransactionState::Abort) => {
                println!("{} Sending abort to {}", self, msg.from);
                // TODO: Send abort
            }
            None => {
                println!("{} Doing transaction", self);
                // TODO: Do transaction

                // after transaction is done
                self.logs.insert(msg.id, TransactionState::Prepared);
                // TODO: Send VoteYes

                // TODO: If there is a reason to abort, send VoteNo instead of VoteYes
            }
        };
        let (tx, rx) = oneshot::channel();
        self.timeout_channels.insert(msg.id, tx);
        let transaction_id = msg.id;

        async move {
            if time::timeout(SLEEP_TIME, rx).await.is_err() {
                println!("[TwoPhaseCommit] Timeout");
                Err(CommitError::Timeout)
            } else {
                println!("[TwoPhaseCommit] Received commit");
                Ok(())
            }
        }.into_actor(self).map(move |r, me, _| {
            if r.is_err() {
                if let Some(TransactionState::Prepared) = me.logs.get(&transaction_id) {
                    println!("{} Timeout while waiting for commit for {}", me, transaction_id);
                    me.logs.insert(transaction_id, TransactionState::Abort);
                    Err(CommitError::Timeout)
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        }).boxed_local()
    }
}
