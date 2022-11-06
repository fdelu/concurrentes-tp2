use std::{io, net::SocketAddr};

use actix_rt::net::{TcpListener, TcpStream};
use tokio::{
    net::ToSocketAddrs,
    task::{spawn, JoinHandle},
};

pub(crate) type OnConnection = Box<dyn Fn(TcpStream, SocketAddr) + Send + 'static>;

pub struct Listener {
    listener: TcpListener,
}

impl Listener {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    pub fn get_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn run(self, on_connection: OnConnection) -> JoinHandle<()> {
        spawn(async move {
            loop {
                match self.listener.accept().await {
                    Ok((stream, addr)) => on_connection(stream, addr),
                    Err(e) => eprintln!("Error accepting connection: {e}"),
                }
            }
        })
    }
}
