use crate::dist_mutex::messages::ack::AckMessage;
use crate::dist_mutex::messages::ok::OkMessage;
use crate::dist_mutex::messages::public::acquire::AcquireMessage;
use crate::dist_mutex::messages::public::release::ReleaseMessage;
use crate::dist_mutex::messages::request::RequestMessage;
use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::{MutexPacket, RequestPacket};
use actix::prelude::*;
use common::AHandler;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::oneshot;

use crate::packet_dispatcher::messages::send_from_mutex::SendFromMutexMessage;
use crate::packet_dispatcher::PacketDispatcherTrait;

pub mod messages;
pub mod packets;
pub mod impl_traits;

const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_secs(10);
const TIME_UNTIL_ERROR: Duration = Duration::from_secs(60);
pub const LOCALHOST: &str = "127.0.0.1";

pub struct DistMutex<P: PacketDispatcherTrait> {
    id: ResourceId,
    dispatcher: Addr<P>,
    lock_timestamp: Option<Timestamp>,

    connected_servers: Vec<ServerId>,
    ack_received: HashSet<ServerId>,
    ok_received: HashSet<ServerId>,
    all_oks_received_channel: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ResourceId {
    id: u32,
}

impl ResourceId {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ServerId {
    pub id: u16,
}

impl ServerId {
    pub fn new(id: u16) -> Self {
        Self { id }
    }
}

pub enum MutexError {
    Timeout,
}

type MutexResult<T> = Result<T, MutexError>;

pub trait DistMutexTrait:
    AHandler<AcquireMessage>
    + AHandler<ReleaseMessage>
    + AHandler<AckMessage>
    + AHandler<OkMessage>
    + AHandler<RequestMessage>
{
}

pub trait MutexCreationTrait<P: PacketDispatcherTrait> {
    fn new(id: ResourceId, dispatcher: Addr<P>) -> Self;
}

impl<P: PacketDispatcherTrait> DistMutexTrait for DistMutex<P> {}

impl<P: PacketDispatcherTrait> MutexCreationTrait<P> for DistMutex<P> {
    fn new(id: ResourceId, dispatcher: Addr<P>) -> Self {
        Self {
            id,
            dispatcher,
            lock_timestamp: None,
            connected_servers: Vec::new(),
            ack_received: HashSet::new(),
            ok_received: HashSet::new(),
            all_oks_received_channel: None,
        }
    }
}

impl<P: PacketDispatcherTrait> Actor for DistMutex<P> {
    type Context = Context<Self>;
}

impl<P: PacketDispatcherTrait> DistMutex<P> {
    pub fn add_connected_server(&mut self, server_id: ServerId) {
        self.connected_servers.push(server_id);
    }

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

    fn broadcast_lock_request(&mut self, timestamp: Timestamp) {
        let lock_request = RequestPacket::new(self.id, timestamp);

        self.connected_servers.iter().for_each(|addr| {
            self.dispatcher
                .try_send(SendFromMutexMessage::new(
                    MutexPacket::Request(lock_request),
                    *addr,
                ))
                .unwrap();
        });
    }

    fn disconnect_servers_with_no_ack(&mut self) {
        self.connected_servers
            .retain(|server_id| self.ack_received.contains(server_id));
    }
}
