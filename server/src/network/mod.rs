use std::cmp::min;
use std::io;
use std::net::SocketAddr;

use actix::Actor;
use actix::Addr;
use actix_rt::task::JoinHandle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use self::connections::ConnectionHandler;
use self::listener::{Listener, OnConnection};
use self::messages::{AddStream, SendPacket};
use self::socket::SocketReceived;
use self::socket::MAX_MESSAGE_SIZE;

mod connections;
mod error;
mod listener;
mod messages;
mod socket;
mod status;

struct ConnectionlessTCP {
    actor: Addr<ConnectionHandler>,
    receiver: UnboundedReceiver<SocketReceived>,
    addr: io::Result<SocketAddr>,
    listener_handle: JoinHandle<()>,
}

impl ConnectionlessTCP {
    fn on_connection(actor: Addr<ConnectionHandler>) -> OnConnection {
        Box::new(move |stream, addr| actor.do_send(AddStream { addr, stream }))
    }

    pub async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.actor
            .send(SendPacket {
                to: addr,
                data: buf[..MAX_MESSAGE_SIZE].to_vec(),
            })
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
        Ok(min(buf.len(), MAX_MESSAGE_SIZE))
    }

    pub async fn recv_from(&mut self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let received = self.receiver.recv().await.ok_or_else(|| {
            io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error receiving: internal channel is closed"),
            )
        })?;
        buf[..received.data.len()].copy_from_slice(&received.data);
        Ok((received.data.len(), received.addr))
    }

    pub async fn bind(addr: SocketAddr) -> io::Result<Self> {
        let (sender, receiver) = unbounded_channel();
        let actor = ConnectionHandler::new(sender).start();

        let listener = Listener::bind(addr).await?;
        let addr = listener.get_addr();

        Ok(ConnectionlessTCP {
            actor: actor.clone(),
            receiver,
            listener_handle: listener.run(Self::on_connection(actor)),
            addr,
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match self.addr {
            Ok(addr) => Ok(addr),
            Err(ref e) => Err(io::Error::new(e.kind(), e.to_string())),
        }
    }
}
