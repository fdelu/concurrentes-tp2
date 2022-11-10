use std::net::SocketAddr;

use actix::Message;
use actix_rt::net::TcpStream;
use tokio::net::ToSocketAddrs;

use crate::network::error::SocketError;

// Public messages

pub use crate::network::socket::ReceivedPacket;

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendPacket {
    pub to: SocketAddr,
    pub data: Vec<u8>,
}

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct Listen<T: ToSocketAddrs> {
    pub bind_to: T,
}

// Private messages

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AddStream {
    pub addr: SocketAddr,
    pub stream: TcpStream,
}
