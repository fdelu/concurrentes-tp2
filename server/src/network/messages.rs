use std::net::SocketAddr;

use actix::Message;
#[cfg(not(test))]
use actix_rt::net::TcpStream;
use serde::Serialize;
#[cfg(test)]
use tests::MockTcpStream as TcpStream;

use crate::network::error::SocketError;

// Public messages

pub use crate::network::socket::ReceivedPacket;

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

#[cfg(test)]
pub mod tests {
    use std::{io, net::SocketAddr};

    use mockall::mock;
    use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

    mock! {
        pub TcpStream {
            pub async fn connect(addr: SocketAddr) -> io::Result<Self>;
            pub fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf);
        }
    }
}
