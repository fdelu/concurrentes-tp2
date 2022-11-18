use crate::two_phase_commit::{TransactionId, TwoPhaseCommit};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct RemoveTransactionMessage {
    pub transaction_id: TransactionId,
}

impl<P: Actor> Handler<RemoveTransactionMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: RemoveTransactionMessage, ctx: &mut Self::Context) -> Self::Result {
        println!(
            "{} Timeout while waiting for transaction {}, aborting it",
            self, msg.transaction_id
        );
        self.abort_transaction(msg.transaction_id, ctx);
    }
}
