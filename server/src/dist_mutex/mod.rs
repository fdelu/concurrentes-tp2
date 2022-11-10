use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::RequestPacket;
use actix::prelude::*;
use common::AHandler;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time;

use crate::network::{ConnectionHandler, SendPacket};

mod messages;
mod packets;

const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_secs(3);
const TIME_UNTIL_ERROR: Duration = Duration::from_secs(60);
pub const LOCALHOST: &str = "127.0.0.1";

pub struct DistMutex<T: TCPActorTrait> {
    server_id: ServerId,
    id: ResourceId,
    socket: Addr<T>,
    lock_timestamp: Option<Timestamp>,
    connected_servers: Vec<ServerId>,
    ack_received: HashSet<ServerId>,
    ok_received: HashSet<ServerId>,
    sleep_handle: Option<JoinHandle<()>>,
    all_oks_received_channel: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ResourceId {
    id: u32,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ServerId {
    pub id: u16,
}

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

pub enum MutexError {
    Timeout,
}

type MutexResult<T> = Result<T, MutexError>;

pub trait TCPActorTrait: AHandler<SendPacket> {}

impl TCPActorTrait for ConnectionHandler {}

impl<T: TCPActorTrait> Actor for DistMutex<T> {
    type Context = Context<Self>;
}

impl<T: TCPActorTrait> DistMutex<T> {
    fn are_all_ok_received(&self) -> bool {
        self.connected_servers
            .iter()
            .all(|server_id| self.ok_received.contains(server_id))
    }

    fn clean_state(&mut self) {
        self.ack_received.clear();
        self.ok_received.clear();
        self.all_oks_received_channel = None;
    }

    fn broadcast_lock_request(&mut self) {
        let lock_request = RequestPacket::new(self.id, self.server_id);

        self.connected_servers.iter().for_each(|addr| {
            self.socket.do_send(SendPacket {
                to: (*addr).into(),
                data: lock_request.into(),
            });
        });
    }
}
