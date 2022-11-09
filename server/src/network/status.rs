use std::net::SocketAddr;

use actix::{dev::ToEnvelope, Actor, Addr, Handler};
use actix_rt::net::TcpStream;
use tokio::{
    task::{spawn, JoinHandle},
    time::Duration,
};

use super::socket::{Socket, SocketEnd, SocketReceived};

pub struct SocketStatus<A>
where
    A: Actor + Handler<SocketEnd> + Handler<SocketReceived>,
    A::Context: ToEnvelope<A, SocketEnd> + ToEnvelope<A, SocketReceived>,
{
    socket: Addr<Socket>,
    cancel_task: Option<JoinHandle<()>>,
    cancel_actor: Addr<A>,
    addr: SocketAddr,
}

const CANCEL_TIMEOUT: Duration = Duration::from_secs(120);

impl<A> SocketStatus<A>
where
    A: Actor + Handler<SocketEnd> + Handler<SocketReceived>,
    A::Context: ToEnvelope<A, SocketEnd> + ToEnvelope<A, SocketReceived>,
{
    pub fn new(
        callback_actor: Addr<A>,
        socket_addr: SocketAddr,
        stream: Option<TcpStream>,
    ) -> SocketStatus<A> {
        let socket = Socket::new(callback_actor.clone(), socket_addr, stream).start();
        let mut this = SocketStatus {
            socket,
            cancel_task: None,
            cancel_actor: callback_actor,
            addr: socket_addr,
        };
        this.restart_timeout();
        this
    }

    pub fn cancel_timeout(&mut self) {
        if let Some(task) = self.cancel_task.take() {
            task.abort();
        }
    }

    pub fn restart_timeout(&mut self) {
        self.cancel_timeout();
        let actor = self.cancel_actor.clone();
        let addr = self.addr;
        self.cancel_task = Some(spawn(async move {
            tokio::time::sleep(CANCEL_TIMEOUT).await;
            actor.do_send(SocketEnd { addr });
        }));
    }

    pub fn get_socket(&self) -> Addr<Socket> {
        self.socket.clone()
    }
}
