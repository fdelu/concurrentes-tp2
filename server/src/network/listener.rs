use std::io;

#[cfg(test)]
use self::tests::MockTcpListener as TcpListener;
use actix::Addr;
#[cfg(not(test))]
use actix_rt::net::TcpListener;
use common::AHandler;
use tokio::{
    net::ToSocketAddrs,
    task::{spawn, JoinHandle},
};

#[cfg(test)]
use mockall::automock;

use super::{error::SocketError, AddStream};

pub struct Listener {
    listener: TcpListener,
}

#[cfg_attr(test, automock)]
impl Listener {
    pub async fn bind<A: ToSocketAddrs + 'static>(addr: A) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
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

#[cfg(test)]
mod tests {
    use crate::network::messages::tests::MockTcpStream as TcpStream;
    use mockall::mock;
    use std::{io, net::SocketAddr};
    use tokio::net::ToSocketAddrs;

    mock! {
        pub TcpListener {
            pub async fn accept(&mut self) -> io::Result<(TcpStream, SocketAddr)>;
            pub fn local_addr(&self) -> io::Result<SocketAddr>;
            pub async fn bind<A: ToSocketAddrs + 'static>(addr: A) -> io::Result<Self>;
        }
    }
}
