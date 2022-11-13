use actix::prelude::*;

use crate::dist_mutex::packets::{ReleasePacket};
use crate::dist_mutex::{DistMutex, MutexResult};
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::packet::PacketType;
use crate::packet_dispatcher::PacketDispatcherTrait;

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct ReleaseMessage;

impl ReleaseMessage {
    pub fn new(_: ReleasePacket) -> Self {
        Self
    }
}

impl<P: PacketDispatcherTrait> Handler<ReleaseMessage> for DistMutex<P> {
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: ReleaseMessage, _: &mut Self::Context) -> Self::Result {
        let dispatcher = self.dispatcher.clone();
        let id = self.id.clone();

        async move {
            let packet = ReleasePacket::new(id);
            dispatcher.send(BroadcastMessage {
                packet_type: PacketType::Mutex,
                data: packet.into(),
            }).await.unwrap();
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}
