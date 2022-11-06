use std::sync::mpsc::{channel, Receiver};

use actix::Actor;
use actix::Addr;
use common::udp_trait::UdpTrait;

use self::connections::ConnectionHandler;
use self::messages::SendPacket;
use self::socket::SocketReceived;

mod connections;
mod error;
mod messages;
mod socket;

struct ConnectionlessTCP {
    actor: Addr<ConnectionHandler>,
    receiver: Receiver<SocketReceived>,
}

impl UdpTrait for ConnectionlessTCP {
    fn send_to(&self, buf: &[u8], addr: std::net::SocketAddr) -> std::io::Result<usize> {
        self.actor.do_send(SendPacket {
            to: addr,
            data: buf.to_vec(),
        });
        Ok(buf.len())
    }

    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, std::net::SocketAddr)> {
        let received = self.receiver.recv().unwrap();
        buf[..received.data.len()].copy_from_slice(&received.data);
        Ok((1, received.addr))
    }

    fn bind<U: std::net::ToSocketAddrs>(addr: U) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let (sender, receiver) = channel();
        let actor = ConnectionHandler::new(sender).start();
        Ok(ConnectionlessTCP { actor, receiver })
    }

    fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        todo!()
    }
}
