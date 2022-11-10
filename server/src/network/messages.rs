use std::net::SocketAddr;

use actix::Message;
use actix_rt::net::TcpStream;

use crate::network::error::SocketError;

// Public messages

pub type RecvOutput = (SocketAddr, Vec<u8>);

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendPacket {
    pub to: SocketAddr,
    pub data: Vec<u8>,
}

#[derive(Message)]
#[rtype(result = "Result<RecvOutput, SocketError>")]
pub struct RecvPacket {}

// Private messages

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AddStream {
    pub addr: SocketAddr,
    pub stream: TcpStream,
}
