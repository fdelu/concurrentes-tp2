use actix::prelude::*;
use crate::packet_dispatcher::messages::send::SendMessage;

use common::AHandler;
use crate::two_phase_commit::{CommitResult, TwoPhaseCommit};

#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct VoteYesMessage {
    pub from: u32,
    pub id: u32,
}

impl<P: AHandler<SendMessage>> Handler<VoteYesMessage> for TwoPhaseCommit<P> {
    type Result = ResponseActFuture<Self, CommitResult<()>>;

    fn handle(&mut self, msg: VoteYesMessage, _ctx: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}