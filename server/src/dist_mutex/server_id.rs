use std::fmt::Display;
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

const PORT: u16 = 8080;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ServerId {
    pub id: u16,
}

impl ServerId {
    pub fn new(id: u16) -> Self {
        Self { id }
    }
}

impl From<ServerId> for SocketAddr {
    fn from(server_id: ServerId) -> Self {
        let ip = format!("127.0.0.{}", server_id.id + 1);
        let port = PORT;
        SocketAddr::new(ip.parse().unwrap(), port)
    }
}

impl From<SocketAddr> for ServerId {
    fn from(socket_addr: SocketAddr) -> Self {
        let ip = socket_addr.ip().to_string();
        let id: u16 = ip.split('.').last().unwrap().parse().unwrap();
        Self { id: id - 1 }
    }
}

impl Display for ServerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ServerId {}]", self.id)
    }
}
