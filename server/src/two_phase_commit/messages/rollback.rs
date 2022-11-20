use crate::two_phase_commit::{TransactionId, TransactionState, TwoPhaseCommit};
use actix::prelude::*;
use tracing::info;

#[derive(Message)]
#[rtype(result = "()")]
pub struct RollbackMessage {
    pub id: TransactionId,
}

impl<P: Actor> Handler<RollbackMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: RollbackMessage, ctx: &mut Self::Context) -> Self::Result {
        info!("{} Received rollback for {}", self, msg.id);

        if let Some((state, _)) = self.logs.get_mut(&msg.id) {
            *state = TransactionState::Abort;
        }
        self.abort_transaction(msg.id, ctx);
    }
}
