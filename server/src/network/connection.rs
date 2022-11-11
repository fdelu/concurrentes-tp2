use std::net::SocketAddr;

use actix::{Actor, Addr};
use actix_rt::net::TcpStream;
use common::AHandler;
use tokio::{
    task::{spawn, JoinHandle},
    time::Duration,
};

use super::socket::{ReceivedPacket, Socket, SocketEnd};

pub struct Connection<A: AHandler<SocketEnd>> {
    socket: Addr<Socket>,
    cancel_task: Option<JoinHandle<()>>,
    end_handler: Addr<A>,
    addr: SocketAddr,
}

const CANCEL_TIMEOUT: Duration = Duration::from_secs(120);

impl<A: AHandler<SocketEnd>> Connection<A> {
    pub fn new<B: AHandler<ReceivedPacket>>(
        end_handler: Addr<A>,
        received_handler: Addr<B>,
        addr: SocketAddr,
        stream: Option<TcpStream>,
    ) -> Self {
        let socket = Socket::new(received_handler, end_handler.clone(), addr, stream);
        let mut this = Connection {
            socket: socket.start(),
            cancel_task: None,
            end_handler,
            addr,
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
        let end_handler = self.end_handler.clone();
        let addr = self.addr;
        self.cancel_task = Some(spawn(async move {
            tokio::time::sleep(CANCEL_TIMEOUT).await;
            end_handler.do_send(SocketEnd { addr });
        }));
    }

    pub fn get_socket(&self) -> Addr<Socket> {
        self.socket.clone()
    }
}
