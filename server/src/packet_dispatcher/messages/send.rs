use actix::prelude::*;

use crate::dist_mutex::server_id::ServerId;
use crate::network::SendPacket;
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
        let socket_addr = self.socket.clone();
        async move {
            match socket_addr
                .send(SendPacket {
                    to: msg.to.into(),
                    data: msg.packet,
                })
                .await?
            {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error sending packet to {}: {}", msg.to, e);
                    Err(e)
                }
            }
        }
        .into_actor(self)
        .map(|res, _act, _ctx| res)
        .boxed_local()
    }
}
