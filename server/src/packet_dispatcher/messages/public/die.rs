use actix::prelude::*;
use tracing::debug;
use crate::PacketDispatcher;

#[derive(Message)]
#[rtype(result = "()")]
pub struct DieMessage;

impl Handler<DieMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, _msg: DieMessage, ctx: &mut Self::Context) -> Self::Result {
        debug!("[PacketDispatcher] Received DieMessage");
        ctx.stop();
    }
}