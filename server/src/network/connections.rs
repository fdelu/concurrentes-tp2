use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use actix_rt::net::TcpStream;
use std::{collections::HashMap, net::SocketAddr};
use tokio::sync::mpsc::UnboundedSender;

use super::{
    error::SocketError,
    messages::{AddStream, SendPacket},
    socket::{Socket, SocketEnd, SocketReceived, SocketSend},
    status::SocketStatus,
};

pub(crate) struct ConnectionHandler {
    connections: HashMap<SocketAddr, SocketStatus<Self>>,
    sender: UnboundedSender<SocketReceived>,
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;
}

impl ConnectionHandler {
    pub(crate) fn new(sender: UnboundedSender<SocketReceived>) -> Self {
        Self {
            connections: HashMap::new(),
            sender,
        }
    }

    async fn send(socket: Addr<Socket>, msg: SendPacket) -> Result<(), SocketError> {
        socket.send(SocketSend { data: msg.data }).await?
    }

    fn create_socket(
        this_actor: Addr<Self>,
        addr: SocketAddr,
        stream: Option<TcpStream>,
    ) -> SocketStatus<Self> {
        let this_actor_clone = this_actor.clone();
        let socket = Socket::new(this_actor, addr, stream).start();
        SocketStatus::new(socket, this_actor_clone, addr)
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
        if let Some(status) = self.connections.get_mut(&msg.addr) {
            status.restart_task();
        }
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
        let status = self
            .connections
            .entry(msg.to)
            .or_insert_with(move || Self::create_socket(this_actor, socket_addr, None));

        status.restart_task();
        let socket = status.get_socket();
        async move { Self::send(socket, msg).await }
            .into_actor(self)
            .boxed_local()
    }
}

impl Handler<AddStream> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddStream, _ctx: &mut Self::Context) -> Self::Result {
        let this_actor = _ctx.address();
        self.connections.insert(
            msg.addr,
            Self::create_socket(this_actor, msg.addr, Some(msg.stream)),
        );
    }
}
