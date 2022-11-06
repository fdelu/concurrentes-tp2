use std::net::SocketAddr;

use actix::Message;

use crate::network::error::SocketError;

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub(crate) struct SendPacket {
    pub to: SocketAddr,
    pub data: Vec<u8>,
}
