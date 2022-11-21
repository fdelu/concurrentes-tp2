use actix::prelude::*;
use tracing::error;

use crate::dist_mutex::server_id::ServerId;
use crate::packet_dispatcher::packet::Packet;
use crate::PacketDispatcher;
use common::socket::SocketError;

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendMessage {
    pub to: ServerId,
    pub packet: Packet,
}

impl Handler<SendMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SendMessage, _ctx: &mut Self::Context) -> Self::Result {
        let id: ServerId = msg.to;
        let fut = self.send_data(id, msg.packet.clone());

        async move {
            match fut.await? {
                Ok(()) => Ok(()),
                Err(e) => {
                    error!("Error sending packet to {}: {}", msg.to, e);
                    Err(e)
                }
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}
