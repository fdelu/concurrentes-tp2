use actix::Actor;
use core::convert::From;
use core::fmt::Display;
use std::net::SocketAddr;

use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::{DistMutex, ResourceId, ServerId};

const PORT: u16 = 8080;

impl From<ServerId> for SocketAddr {
    fn from(server_id: ServerId) -> Self {
        let ip = format!("127.0.0.{}", server_id.id + 1);
        let port = PORT;
        SocketAddr::new(ip.parse().unwrap(), port)
    }
}

impl From<&[u8]> for ServerId {
    fn from(bytes: &[u8]) -> Self {
        let id = u16::from_be_bytes(bytes.try_into().unwrap());
        Self { id }
    }
}

impl From<ServerId> for [u8; 2] {
    fn from(server_id: ServerId) -> Self {
        server_id.id.to_be_bytes()
    }
}

impl From<SocketAddr> for ServerId {
    fn from(addr: SocketAddr) -> Self {
        let id: u16 = addr
            .ip()
            .to_string()
            .split('.')
            .last()
            .unwrap()
            .parse()
            .unwrap();

        Self { id: id - 1 }
    }
}

impl Display for ServerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ServerId {}]", self.id)
    }
}

impl From<&[u8]> for ResourceId {
    fn from(bytes: &[u8]) -> Self {
        let id = u32::from_be_bytes(bytes.try_into().unwrap());
        Self { id }
    }
}

impl From<ResourceId> for [u8; 4] {
    fn from(id: ResourceId) -> Self {
        id.id.to_be_bytes()
    }
}

impl Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl<P: Actor> Display for DistMutex<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Mutex {}]", self.id)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.time())
    }
}
