use std::{collections::HashMap, mem::replace, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use actix_rt::{net::TcpStream, task::JoinHandle};
use tokio::{
    net::ToSocketAddrs,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

mod error;
mod listener;
mod messages;
mod socket;
mod status;

pub use self::messages::*;
use self::{
    error::SocketError,
    listener::{Listener, OnConnection},
    socket::{Socket, SocketEnd, SocketReceived, SocketSend},
    status::SocketStatus,
};

pub struct ConnectionHandler {
    connections: HashMap<SocketAddr, SocketStatus<Self>>,
    incoming_buffer: UnboundedSender<RecvOutput>,
    recv_queue: oneshot::Receiver<UnboundedReceiver<RecvOutput>>,
    join_listener: JoinHandle<()>,
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.join_listener.abort();
    }
}

impl ConnectionHandler {
    fn initialize(
        ctx: &mut Context<Self>,
        listener: Listener,
        incoming_buffer: UnboundedSender<RecvOutput>,
        recv_queue: oneshot::Receiver<UnboundedReceiver<RecvOutput>>,
    ) -> Self {
        let actor_addr = ctx.address();
        Self {
            connections: HashMap::new(),
            incoming_buffer,
            recv_queue,
            join_listener: listener.run(Self::on_connection(actor_addr)),
        }
    }

    pub async fn bind<T: ToSocketAddrs>(address: T) -> Result<Addr<Self>, SocketError> {
        let listener = Listener::bind(address).await?;
        let (incoming_tx, incoming_rx) = unbounded_channel();
        let (recv_tx, recv_rx) = oneshot::channel();
        recv_tx.send(incoming_rx).map_err(|_| {
            SocketError::new("Failed to send buffer receiver through initial oneshot channel")
        })?;

        Ok(Self::create(move |ctx| {
            Self::initialize(ctx, listener, incoming_tx, recv_rx)
        }))
    }

    fn on_connection(actor: Addr<ConnectionHandler>) -> OnConnection {
        Box::new(move |stream, addr| actor.do_send(AddStream { addr, stream }))
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

// Public messages

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
        async move { socket.send(SocketSend { data: msg.data }).await? }
            .into_actor(self)
            .boxed_local()
    }
}

impl Handler<RecvPacket> for ConnectionHandler {
    type Result = ResponseActFuture<Self, Result<RecvOutput, SocketError>>;

    fn handle(&mut self, _msg: RecvPacket, _ctx: &mut Context<Self>) -> Self::Result {
        let (tx, rx) = oneshot::channel();
        let recv_queue = replace(&mut self.recv_queue, rx);

        async move {
            let mut receiver = recv_queue.await?;
            let out = receiver
                .recv()
                .await
                .ok_or_else(|| SocketError::new("Error receiving: internal channel is closed"));
            tx.send(receiver).map_err(|_| {
                SocketError::new("Failed to send buffer receiver through oneshot channel")
            })?;
            out
        }
        .into_actor(self)
        .boxed_local()
    }
}

// Private Messages

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
        if let Err(e) = self.incoming_buffer.send((msg.addr, msg.data)) {
            println!("Error sending through channel: {e}");
        }
    }
}
