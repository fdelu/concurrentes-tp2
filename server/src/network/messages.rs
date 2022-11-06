use std::net::SocketAddr;

use actix::Message;
use actix_rt::net::TcpStream;

use crate::network::error::SocketError;

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub(crate) struct SendPacket {
    pub to: SocketAddr,
    pub data: Vec<u8>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AddStream {
    pub addr: SocketAddr,
    pub stream: TcpStream,
}
