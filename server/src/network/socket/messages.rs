use actix::Message;
use std::net::SocketAddr;
use tokio::sync::mpsc::error::SendError;

#[derive(Message)]
#[rtype(result = "Result<(), SendError<Vec<u8>>>")]
pub struct SocketSend {
    pub data: Vec<u8>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SocketReceived {
    pub data: Vec<u8>,
    pub addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SocketEnd {
    pub addr: SocketAddr,
}
