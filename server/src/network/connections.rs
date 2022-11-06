use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use std::{collections::HashMap, net::SocketAddr, sync::mpsc::Sender};

use super::{
    error::SocketError,
    messages::SendPacket,
    socket::{Socket, SocketEnd, SocketReceived, SocketSend},
};

pub(crate) struct ConnectionHandler {
    connections: HashMap<SocketAddr, Addr<Socket>>,
    sender: Sender<SocketReceived>,
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;
}

impl ConnectionHandler {
    pub(crate) fn new(sender: Sender<SocketReceived>) -> Self {
        Self {
            connections: HashMap::new(),
            sender,
        }
    }

    async fn send(socket: Addr<Socket>, msg: SendPacket) -> Result<(), SocketError> {
        Ok(socket.send(SocketSend { data: msg.data }).await??)
    }
}

impl Handler<SocketEnd> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: SocketEnd, _ctx: &mut Self::Context) {
        self.connections.remove(&msg.addr);
    }
}

impl Handler<SocketReceived> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: SocketReceived, _ctx: &mut Self::Context) {
        if let Err(e) = self.sender.send(msg) {
            println!("Error sending through channel: {e}");
        }
    }
}

impl Handler<SendPacket> for ConnectionHandler {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SendPacket, _ctx: &mut Context<Self>) -> Self::Result {
        let this_actor = _ctx.address();
        let socket_addr = msg.to;
        let socket = self
            .connections
            .entry(msg.to)
            .or_insert_with(move || Socket::new(this_actor, socket_addr).start())
            .clone();

        async move { Self::send(socket, msg).await }
            .into_actor(self)
            .boxed_local()
    }
}
