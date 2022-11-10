use crate::dist_mutex::packets::OkPacket;
use crate::dist_mutex::{DistMutex, ServerId};
use crate::packet_dispatcher::PacketDispatcherTrait;
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct OkMessage {
    sender: ServerId,
}

impl OkMessage {
    pub fn new(packet: OkPacket) -> Self {
        Self {
            sender: packet.sender(),
        }
    }
}

impl<P: PacketDispatcherTrait> Handler<OkMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: OkMessage, _ctx: &mut Self::Context) {
        self.ok_received.insert(msg.sender);
        if self.are_all_ok_received() {
            self.all_oks_received_channel.take().unwrap().send(()).unwrap();
        }
    }
}
