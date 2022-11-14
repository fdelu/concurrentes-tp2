use crate::dist_mutex::packets::OkPacket;
use crate::dist_mutex::{DistMutex, ServerId};
use crate::packet_dispatcher::PacketDispatcherTrait;
use actix::prelude::*;
use std::collections::HashSet;

#[derive(Message)]
#[rtype(result = "()")]
pub struct OkMessage {
    from: ServerId,
    connected_servers: HashSet<ServerId>,
}

impl OkMessage {
    pub fn new(from: ServerId, connected_servers: HashSet<ServerId>, _: OkPacket) -> Self {
        Self {
            from,
            connected_servers,
        }
    }
}

impl<P: Actor> Handler<OkMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: OkMessage, _ctx: &mut Self::Context) {
        println!("{} Received ok from {}", self, msg.from);

        self.ok_received.insert(msg.from);

        if self.ok_received.is_superset(&msg.connected_servers) {
            println!("{} All ok received", self);
            self.all_oks_received_channel
                .take()
                .unwrap()
                .send(())
                .unwrap();
        }
    }
}
