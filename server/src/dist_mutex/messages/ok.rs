use crate::dist_mutex::packets::OkPacket;
use crate::dist_mutex::{DistMutex, ServerId};
use crate::packet_dispatcher::PacketDispatcherTrait;
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct OkMessage {
    from: ServerId,
}

impl OkMessage {
    pub fn new(from: ServerId, _: OkPacket) -> Self {
        Self { from }
    }
}

impl<P: PacketDispatcherTrait> Handler<OkMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: OkMessage, _ctx: &mut Self::Context) {
        self.ok_received.insert(msg.from);
        if self.are_all_ok_received() {
            println!(
                "{} All ok received ({:?}) ({:?})",
                self, self.ok_received, self.connected_servers
            );
            self.all_oks_received_channel
                .take()
                .unwrap()
                .send(())
                .unwrap();
        }
    }
}
