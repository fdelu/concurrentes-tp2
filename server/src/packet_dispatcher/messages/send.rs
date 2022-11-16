use actix::prelude::*;

use crate::network::{SendPacket, SocketError};
use crate::packet_dispatcher::packet::PacketType;
use crate::{PacketDispatcher, ServerId};

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendMessage {
    pub to: ServerId,
    pub data: Vec<u8>,
    pub packet_type: PacketType,
}

impl Handler<SendMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, mut msg: SendMessage, _ctx: &mut Self::Context) -> Self::Result {
        let mut data = vec![u8::from(msg.packet_type)];
        data.append(&mut msg.data);
        let socket_addr = self.socket.clone();
        async move {
            match socket_addr
                .send(SendPacket {
                    to: msg.to.into(),
                    data,
                })
                .await?
            {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!(
                        "Error sending packet of type {:?} to {}: {}",
                        msg.packet_type, msg.to, e
                    );
                    Err(e)
                }
            }
        }
        .into_actor(self)
        .map(|res, _act, _ctx| res)
        .boxed_local()
    }
}
