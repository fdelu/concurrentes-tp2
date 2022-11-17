use std::net::SocketAddr;

use actix::Message;
#[cfg(not(test))]
use actix_rt::net::TcpStream;
#[cfg(test)]
use common::socket::test_util::MockTcpStream as TcpStream;
use serde::Serialize;

use common::socket::SocketError;

// Public messages

pub use common::socket::ReceivedPacket;

#[derive(Message, PartialEq, Eq, Clone, Debug)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendPacket<T: Serialize> {
    pub to: SocketAddr,
    pub data: T,
}

#[derive(Message, PartialEq, Eq, Clone, Debug)]
#[rtype(result = "Result<(), SocketError>")]
pub struct Listen {}

// Private messages

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AddStream {
    pub addr: SocketAddr,
    pub stream: TcpStream,
}
