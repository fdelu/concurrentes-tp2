use actix::prelude::*;
use crate::two_phase_commit::{CommitResult, TransactionState, TwoPhaseCommit};

use common::AHandler;
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::two_phase_commit::packets::PreparePacket;

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct SubmitMessage {
    pub user_id: u32,
    pub new_value: u32,
}

impl<P: AHandler<BroadcastMessage>> Handler<SubmitMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<()>>;

    fn handle(&mut self, msg: SubmitMessage, _ctx: &mut Self::Context) -> Self::Result {
        let prepare_packet = PreparePacket::new(msg.user_id, msg.new_value);
        let id = prepare_packet.transaction_id;
        match self.logs.get(&id) {
            None | Some(TransactionState::Prepared) => {
                // TODO: Broadcast prepare
            },
            Some(TransactionState::Commit) => {
                // TODO: Broadcast commit
            },
            Some(TransactionState::Abort) => {
                // TODO: Broadcast abort
            },
        }
        async move {
            Ok(())
        }.into_actor(self).boxed_local()
    }
}
