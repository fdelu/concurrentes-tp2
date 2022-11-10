use crate::dist_mutex::{DistMutex, ServerId, TCPActorTrait};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct AckMessage {
    pub server_id: ServerId,
}

impl<T: TCPActorTrait> Handler<AckMessage> for DistMutex<T> {
    type Result = ();

    fn handle(&mut self, msg: AckMessage, _ctx: &mut Self::Context) {
        self.ack_received.insert(msg.server_id);
    }
}
