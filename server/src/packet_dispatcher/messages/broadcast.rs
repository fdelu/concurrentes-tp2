use actix::prelude::*;
use crate::{PacketDispatcher};
use crate::network::error::SocketError;
use crate::network::SendPacket;
use crate::packet_dispatcher::packet::PacketType;

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct BroadcastMessage {
    pub packet_type: PacketType,
    pub data: Vec<u8>,
}

impl Handler<BroadcastMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: BroadcastMessage, _ctx: &mut Self::Context) -> Self::Result {
        let mut packet = vec![msg.packet_type as u8];
        packet.extend_from_slice(&msg.data);

        let futures: Vec<_> = self.get_connected_servers().iter().map(|server_id| {
            self.send_data(*server_id, packet.clone())
        }).collect();
        
        async move {
            for future in futures {
                future.await??;
            }
            Ok(())
        }.into_actor(self).boxed_local()
    }
}

