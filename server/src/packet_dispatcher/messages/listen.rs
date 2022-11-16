use crate::network::Listen;
use crate::network::SocketError;
use crate::PacketDispatcher;
use actix::prelude::*;

impl Handler<Listen> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: Listen, _ctx: &mut Self::Context) -> Self::Result {
        let socket_actor_addr = self.socket.clone();

        async move {
            match socket_actor_addr.send(msg).await {
                Ok(_) => Ok(()),
                Err(e) => Err(SocketError::from(e)),
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}
