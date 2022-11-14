use crate::dist_mutex::packets::AckPacket;
use crate::dist_mutex::{DistMutex, ServerId};
use crate::packet_dispatcher::PacketDispatcherTrait;
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct AckMessage {
    from: ServerId,
}

impl AckMessage {
    pub fn new(from: ServerId, _: AckPacket) -> Self {
        Self { from }
    }
}

impl<P: Actor> Handler<AckMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: AckMessage, _ctx: &mut Self::Context) {
        self.ack_received.insert(msg.from);
    }
}
