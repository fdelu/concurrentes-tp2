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

/// Mensaje del [ConnectionHandler](super::ConnectionHandler)
/// para enviar un paquete a una conexión específica.
#[derive(Message, PartialEq, Eq, Clone, Debug)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendPacket<T: Serialize> {
    pub to: SocketAddr,
    pub data: T,
}

/// Mensaje del [ConnectionHandler](super::ConnectionHandler)
/// para inicializar el [TcpListener](tokio::net::TcpListener)
/// y comenzar a aceptar conexiones.
#[derive(Message, PartialEq, Eq, Clone, Debug)]
#[rtype(result = "Result<(), SocketError>")]
pub struct Listen {}

// Private messages

/// Mensaje del [ConnectionHandler](super::ConnectionHandler)
/// para añadir una nueva conexión a la lista de conexiones
/// con un [TcpStream](tokio::net::TcpStream) dado.
#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AddStream {
    pub addr: SocketAddr,
    pub stream: TcpStream,
}
