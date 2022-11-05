use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use actix_rt::net::TcpStream;
use std::{collections::HashMap, net::SocketAddr, sync::mpsc::Sender};

use super::socket::{Socket, SocketReceived, SocketSend};

pub(crate) struct ConnectionHandler {
    connections: HashMap<SocketAddr, Addr<Socket>>,
    sender: Sender<SocketReceived>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct Send {
    pub to: SocketAddr,
    pub data: Vec<u8>,
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
}

impl Handler<Send> for ConnectionHandler {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: Send, _ctx: &mut Context<Self>) -> Self::Result {
        //Box::pin(self.send_message(msg).into_actor(self))
        let data = msg.data;
        let to = msg.to;
        let my_addr = _ctx.address();
        if let Some(socket) = self.connections.get_mut(&msg.to) {
            let socket_addr = socket.clone();
            Box::pin(
                async move {
                    socket_addr.send(SocketSend { data }).await.unwrap();
                }
                .into_actor(self),
            )
        } else {
            Box::pin(
                async move {
                    let connection = TcpStream::connect(to).await.unwrap();
                    let addr = to;
                    let (reader, writer) = connection.into_split();
                    let socket = Socket::new(my_addr, reader, writer, addr);
                    let socket_addr = socket.start();
                    socket_addr.send(SocketSend { data }).await.unwrap();
                    socket_addr
                }
                .into_actor(self)
                .map(move |socket_addr, act, _ctx| {
                    act.connections.insert(to, socket_addr);
                }),
            )
        }
    }
}

impl Handler<SocketReceived> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: SocketReceived, _ctx: &mut Context<Self>) -> Self::Result {
        self.sender.send(msg).unwrap();
    }
}
