use std::time::Duration;

use actix::prelude::*;
use tokio::sync::oneshot;
use tokio::time;

use common::AHandler;

use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::two_phase_commit::packets::{PreparePacket, Transaction};
use crate::two_phase_commit::{CommitError, CommitResult, TransactionId, TwoPhaseCommit};

const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_millis(5000);

#[derive(Message)]
#[rtype(result = "CommitResult<bool>")]
pub struct CommitRequestMessage {
    pub id: TransactionId,
    pub transaction: Transaction,
}

impl<P: AHandler<BroadcastMessage>> Handler<CommitRequestMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<bool>>;

    fn handle(&mut self, msg: CommitRequestMessage, ctx: &mut Self::Context) -> Self::Result {
        let prepare_packet = PreparePacket::new(msg.id, msg.transaction);
        let id = prepare_packet.id;
        if !self.prepare_transaction(id, prepare_packet.transaction, ctx) {
            return Box::pin(async { Ok(false) }.into_actor(self));
        }

        self.broadcast_prepare(prepare_packet);

        let (tx, rx) = oneshot::channel();
        self.coordinator_timeouts.insert(id, tx);

        async move {
            let r = time::timeout(TIME_UNTIL_DISCONNECT_POLITIC, rx).await;
            r.map_err(|_| CommitError::Timeout).map(|r| r.unwrap())
        }
        .into_actor(self)
        .boxed_local()
    }
}
