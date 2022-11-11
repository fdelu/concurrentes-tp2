use crate::network::error::SocketError;
use crate::network::Listen;
use crate::PacketDispatcher;
use actix::prelude::*;
use std::borrow::Borrow;
use tokio::net::ToSocketAddrs;

impl<T: ToSocketAddrs + Send + 'static> Handler<Listen<T>> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: Listen<T>, _ctx: &mut Self::Context) -> Self::Result {
        let socket_actor_addr = self.socket.clone();

        async move {
            // FIXME: bubble up error
            socket_actor_addr.send(msg).await;
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}
