use std::{collections::HashMap, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use actix_rt::{net::TcpStream, task::JoinHandle};
use tokio::net::ToSocketAddrs;

mod connection;
mod error;
mod listener;
mod messages;
mod socket;

pub use self::messages::*;
pub(crate) use self::socket::ReceivedPacket;

use self::{
    connection::Connection,
    error::SocketError,
    listener::Listener,
    socket::{Socket, SocketEnd, SocketSend},
};
use common::AHandler;

pub struct ConnectionHandler<A: AHandler<ReceivedPacket>> {
    connections: HashMap<SocketAddr, Connection<Self>>,
    received_handler: Addr<A>,
    join_listener: Option<JoinHandle<()>>,
}

impl<A: AHandler<ReceivedPacket>> ConnectionHandler<A> {
    fn new(received_handler: Addr<A>) -> Self {
        Self {
            connections: HashMap::new(),
            received_handler,
            join_listener: None,
        }
    }

    fn create_connection(
        this: Addr<Self>,
        receiver: Addr<A>,
        addr: SocketAddr,
        stream: Option<TcpStream>,
    ) -> Connection<Self> {
        let socket = Socket::new(receiver, this.clone(), addr, stream);
        Connection::new(this, socket)
    }

    fn get_connection(&mut self, this: Addr<Self>, addr: SocketAddr) -> &mut Connection<Self> {
        let receiver = self.received_handler.clone();
        let connection = self
            .connections
            .entry(addr)
            .or_insert_with(move || Self::create_connection(this, receiver, addr, None));
        connection
    }
}

impl<A: AHandler<ReceivedPacket>> Actor for ConnectionHandler<A> {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.join_listener.take().map(|join| join.abort());
    }
}

// Public messages

impl<A: AHandler<ReceivedPacket>> Handler<SendPacket> for ConnectionHandler<A> {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SendPacket, ctx: &mut Context<Self>) -> Self::Result {
        let connection = self.get_connection(ctx.address(), msg.to);
        connection.restart_timeout();
        let socket = connection.get_socket();

        async move { socket.send(SocketSend { data: msg.data }).await? }
            .into_actor(self)
            .boxed_local()
    }
}

impl<A: AHandler<ReceivedPacket>, T: ToSocketAddrs + 'static> Handler<Listen<T>>
    for ConnectionHandler<A>
{
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: Listen<T>, _ctx: &mut Context<Self>) -> Self::Result {
        if (self.join_listener.is_some()) {
            return async { Err(SocketError::new("Already listening for new connection")) }
                .into_actor(self)
                .boxed_local();
        }

        async move { Listener::bind(msg.bind_to).await }
            .into_actor(self)
            .map(|listener, this, ctx| {
                this.join_listener = Some(listener?.run(ctx.address()));
                Ok(())
            })
            .boxed_local()
    }
}

// Private Messages

impl<A: AHandler<ReceivedPacket>> Handler<AddStream> for ConnectionHandler<A> {
    type Result = ();

    fn handle(&mut self, msg: AddStream, ctx: &mut Self::Context) -> Self::Result {
        let connection = Self::create_connection(
            ctx.address(),
            self.received_handler.clone(),
            msg.addr,
            Some(msg.stream),
        );
        self.connections.insert(msg.addr, connection);
    }
}

impl<A: AHandler<ReceivedPacket>> Handler<SocketEnd> for ConnectionHandler<A> {
    type Result = ();

    fn handle(&mut self, msg: SocketEnd, _ctx: &mut Self::Context) {
        self.connections.remove(&msg.addr);
    }
}
