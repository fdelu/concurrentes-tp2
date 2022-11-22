use actix::Message;
use serde::{de::DeserializeOwned, Serialize};
use std::net::SocketAddr;
use tokio::sync::{mpsc::error::SendError, oneshot};

use super::SocketError;

// Public messages

/// Mensaje para enviar un paquete a través del [Socket](super::Socket).
#[derive(Message, PartialEq, Eq, Clone, Debug)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SocketSend<T: Serialize> {
    pub data: T,
}

/// Mensaje que envía el [Socket](super::Socket) al recibir un paquete.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ReceivedPacket<T: DeserializeOwned> {
    pub data: T,
    pub addr: SocketAddr,
}

/// Mensaje que envía el [Socket](super::Socket) al desconectarse.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SocketEnd {
    pub addr: SocketAddr,
}

// Private messages

#[derive(Message, Debug)]
#[rtype(result = "Result<(), SendError<Vec<u8>>>")]
pub(crate) struct WriterSend<T: Serialize> {
    pub data: T,
    pub result: Option<oneshot::Sender<Result<(), SocketError>>>,
}
