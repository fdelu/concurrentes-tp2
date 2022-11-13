use std::net::SocketAddr;
use actix::prelude::*;

use crate::packet_dispatcher::packet::PacketType;
use crate::{PacketDispatcher, ServerId};
use crate::network::error::SocketError;
use crate::network::SendPacket;

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendMessage {
    pub to: ServerId,
    pub data: Vec<u8>,
    pub packet_type: PacketType
}

impl Handler<SendMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, mut msg: SendMessage, ctx: &mut Self::Context) -> Self::Result {
        println!("Sending message to {} (type {:?}, addr: {})", msg.to, msg.packet_type, SocketAddr::from(msg.to));

        let mut data = vec![msg.packet_type as u8];
        data.append(&mut msg.data);
        let socket_addr = self.socket.clone();
        async move {
            match socket_addr.send(SendPacket {
            to: msg.to.into(),
            data
            }).await? {
                Ok(_) => {
                    println!("Sent message of type {:?} to {}", msg.packet_type, msg.to);
                    Ok(())
                },
                Err(e) => {
                    println!("Error sending packet of type {:?} to {}: {}", msg.packet_type, msg.to, e);
                    Err(e)
                }
            }
        }.into_actor(self).map(|res, _act, _ctx| {
            res
        }).boxed_local()
    }
}