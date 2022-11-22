use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ServerId {
    pub ip: IpAddr,
}

impl ServerId {
    pub fn new(ip: IpAddr) -> Self {
        Self { ip }
    }

    pub fn get_socket_addr(&self, port: u16) -> SocketAddr {
        SocketAddr::new(self.ip, port)
    }

    pub fn to_number(&self) -> u16 {
        self.ip.to_string().split('.').last().unwrap().parse().unwrap()
    }
}

impl From<SocketAddr> for ServerId {
    fn from(addr: SocketAddr) -> Self {
        Self::new(addr.ip())
    }
}

impl Display for ServerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ServerId {}]", self.ip)
    }
}
