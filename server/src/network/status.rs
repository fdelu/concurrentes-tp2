use std::net::SocketAddr;

use actix::{dev::ToEnvelope, Actor, Addr, Handler};
use tokio::task::{spawn, JoinHandle};

use super::socket::{Socket, SocketEnd};

pub struct SocketStatus<A>
where
    A: Actor + Handler<SocketEnd>,
    A::Context: ToEnvelope<A, SocketEnd>,
{
    socket: Addr<Socket>,
    cancel_task: Option<JoinHandle<()>>,
    cancel_actor: Addr<A>,
    addr: SocketAddr,
}

const CANCEL_TIMEOUT_SECONDS: u64 = 120;

impl<A> SocketStatus<A>
where
    A: Actor + Handler<SocketEnd>,
    A::Context: ToEnvelope<A, SocketEnd>,
{
    pub fn new(socket: Addr<Socket>, cancel_addr: Addr<A>, addr: SocketAddr) -> Self {
        Self {
            socket,
            cancel_task: None,
            cancel_actor: cancel_addr,
            addr,
        }
    }

    pub fn cancel_task(&mut self) {
        if let Some(task) = self.cancel_task.take() {
            task.abort();
        }
    }

    pub fn restart_task(&mut self) {
        self.cancel_task();
        let actor = self.cancel_actor.clone();
        let addr = self.addr;
        self.cancel_task = Some(spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(CANCEL_TIMEOUT_SECONDS)).await;
            actor.do_send(SocketEnd { addr });
        }));
    }

    pub fn get_socket(&self) -> Addr<Socket> {
        self.socket.clone()
    }
}
