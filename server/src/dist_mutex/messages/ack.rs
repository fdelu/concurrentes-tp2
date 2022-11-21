use crate::dist_mutex::{DistMutex, ServerId};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct AckMessage {
    pub from: ServerId,
}

impl<P: Actor> Handler<AckMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: AckMessage, _ctx: &mut Self::Context) {
        self.ack_received.insert(msg.from);
    }
}
