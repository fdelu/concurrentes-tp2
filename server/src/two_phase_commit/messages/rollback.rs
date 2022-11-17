use crate::two_phase_commit::{TransactionId, TransactionState, TwoPhaseCommit};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct RollbackMessage {
    pub id: TransactionId,
}

impl<P: Actor> Handler<RollbackMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: RollbackMessage, _ctx: &mut Self::Context) -> Self::Result {
        println!("{} Received rollback for {}", self, msg.id);
        self.stakeholder_timeouts
            .remove(&msg.id)
            .map(|tx| tx.send(()));

        self.logs.insert(msg.id, TransactionState::Abort);
        self.abort_transaction(msg.id);
    }
}
