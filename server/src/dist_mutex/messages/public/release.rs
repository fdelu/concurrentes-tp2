use actix::prelude::*;
use crate::dist_mutex::{DistMutex, MutexResult, TCPActorTrait};
use crate::network::SendPacket;

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct ReleaseMessage;

/*
impl<T: TCPActorTrait> Handler<ReleaseMessage> for DistMutex<T> {
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: ReleaseMessage, _: &mut Self::Context) -> Self::Result {
        self.connected_servers.iter().for_each(|server_id| {
            self.socket.try_send(SendPacket {
                to: *server_id,
                data:

            });
        });

    }
}

 */