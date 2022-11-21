use actix::prelude::*;
use tracing::{debug, trace};
use crate::PacketDispatcher;

#[derive(Message)]
#[rtype(result = "()")]
pub struct DieMessage;

impl Handler<DieMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, _msg: DieMessage, _: &mut Self::Context) -> Self::Result {
        debug!("[PacketDispatcher] Received DieMessage");
        self.mutexes.values().for_each(|mutex| {
            mutex.do_send(DieMessage);
        });
    }
}