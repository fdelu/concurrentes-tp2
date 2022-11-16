use actix::prelude::*;

use crate::network::SocketError;
use crate::packet_dispatcher::packet::Packet;
use crate::PacketDispatcher;

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct BroadcastMessage {
    pub packet: Packet,
}

impl Handler<BroadcastMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: BroadcastMessage, _ctx: &mut Self::Context) -> Self::Result {
        println!(
            "Broadcasting to {} servers",
            self.get_connected_servers().len()
        );
        println!("Connected servers: {:?}", self.get_connected_servers());

        let futures: Vec<_> = self
            .get_connected_servers()
            .iter()
            .map(|server_id| {
                println!("Sending to {}", server_id);
                self.send_data(*server_id, serde_json::to_vec(&msg.packet).unwrap())
            })
            .collect();

        async move {
            for future in futures {
                future.await??;
            }
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}
