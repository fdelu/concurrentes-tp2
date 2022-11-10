use std::{io, net::SocketAddr};

use actix::Addr;
use actix_rt::net::TcpListener;
use common::AHandler;
use tokio::{
    net::ToSocketAddrs,
    task::{spawn, JoinHandle},
};

use super::{error::SocketError, AddStream};

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

    async fn add_connection<A: AHandler<AddStream>>(
        listener: &mut TcpListener,
        handler: &Addr<A>,
    ) -> Result<(), SocketError> {
        let (stream, addr) = listener.accept().await?;
        handler.send(AddStream { stream, addr }).await?;
        Ok(())
    }

    pub(crate) fn run<A: AHandler<AddStream>>(mut self, add_handler: Addr<A>) -> JoinHandle<()> {
        spawn(async move {
            loop {
                if let Err(e) = Self::add_connection(&mut self.listener, &add_handler).await {
                    eprintln!("Error accepting connection: {e}");
                }
            }
        })
    }
}
