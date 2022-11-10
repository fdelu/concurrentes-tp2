use actix::prelude::*;

use crate::dist_mutex::packets::{MutexPacket, ReleasePacket};
use crate::dist_mutex::{DistMutex, MutexResult};
use crate::packet_dispatcher::messages::send_from_mutex::SendFromMutexMessage;
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
        let connected_servers = self.connected_servers.clone();
        let socket = self.dispatcher.clone();
        let id = self.id;
        async move {
            for server_id in connected_servers {
                let packet = ReleasePacket::new(id);
                let r = socket
                    .send(SendFromMutexMessage::new(
                        MutexPacket::Release(packet),
                        server_id,
                    ))
                    .await;
                if let Err(e) = r {
                    println!("Error sending release packet: {}", e);
                }
            }
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}
