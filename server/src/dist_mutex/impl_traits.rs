use core::convert::From;
use core::fmt::Display;
use std::net::SocketAddr;
use crate::dist_mutex::{DistMutex, LOCALHOST, ResourceId, ServerId};
use crate::dist_mutex::messages::Timestamp;
use crate::packet_dispatcher::PacketDispatcherTrait;

impl From<ServerId> for SocketAddr {
    fn from(server_id: ServerId) -> Self {
        SocketAddr::new(LOCALHOST.parse().unwrap(), server_id.id)
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
        Self { id: addr.port() }
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

impl<P: PacketDispatcherTrait> Display for DistMutex<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Mutex {}]", self.id)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.time())
    }
}
