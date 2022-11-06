use actix::Message;
use std::net::SocketAddr;
use tokio::sync::{mpsc::error::SendError, oneshot};

use crate::network::error::SocketError;

#[derive(Message, Debug)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SocketSend {
    pub data: Vec<u8>,
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), SendError<Vec<u8>>>")]
pub(crate) struct WriterSend {
    pub data: Vec<u8>,
    pub result: Option<oneshot::Sender<Result<(), SocketError>>>,
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
