use crate::dist_mutex::packets::AckPacket;
use crate::dist_mutex::{DistMutex, ServerId};
use crate::packet_dispatcher::PacketDispatcherTrait;
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct AckMessage {
    acker: ServerId,
}

impl AckMessage {
    pub fn new(packet: AckPacket) -> Self {
        Self {
            acker: packet.acker(),
        }
    }
}

impl<P: PacketDispatcherTrait> Handler<AckMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: AckMessage, _ctx: &mut Self::Context) {
        self.ack_received.insert(msg.acker);
    }
}
