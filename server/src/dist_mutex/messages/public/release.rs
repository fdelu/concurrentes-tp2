use actix::prelude::*;

use crate::dist_mutex::packets::ReleasePacket;
use crate::dist_mutex::{DistMutex, MutexResult, TCPActorTrait};
use crate::network::SendPacket;

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct ReleaseMessage;

impl<T: TCPActorTrait> Handler<ReleaseMessage> for DistMutex<T> {
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: ReleaseMessage, _: &mut Self::Context) -> Self::Result {
        let connected_servers = self.connected_servers.clone();
        let socket = self.socket.clone();
        let id = self.id;
        async move {
            for server_id in connected_servers {
                let packet = ReleasePacket::new(id);
                let r = socket
                    .send(SendPacket {
                        to: server_id.into(),
                        data: packet.into(),
                    })
                    .await;
                if let Err(e) = r {
                    println!("Error sending release packet: {}", e);
                }
            }
        }
        .into_actor(self)
        .boxed_local();

        self.connected_servers.iter().for_each(|server_id| {
            self.socket
                .try_send(SendPacket {
                    to: (*server_id).into(),
                    data: ReleasePacket::new(self.id).into(),
                })
                .unwrap();
        });
        async { Ok(()) }.into_actor(self).boxed_local()
    }
}
