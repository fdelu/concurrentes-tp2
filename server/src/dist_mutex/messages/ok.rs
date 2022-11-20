use crate::dist_mutex::packets::OkPacket;
use crate::dist_mutex::{DistMutex, ServerId};
use actix::prelude::*;
use std::collections::HashSet;
use tracing::{debug, info};

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
        debug!("{} Received ok from {}", self, msg.from);
        info!("Connected servers: {:?}", msg.connected_servers);

        self.ok_received.insert(msg.from);

        if self.ok_received.is_superset(&msg.connected_servers) {
            debug!("{} All ok received", self);
            if let Some(ch) = self.all_oks_received_channel.take() {
                ch.send(()).unwrap();
            }
        }
    }
}
