use crate::dist_mutex::messages::ack::AckMessage;
use crate::dist_mutex::messages::ok::OkMessage;
use crate::dist_mutex::messages::public::acquire::AcquireMessage;
use crate::dist_mutex::messages::public::release::ReleaseMessage;
use crate::dist_mutex::messages::request::RequestMessage;
use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::{RequestPacket};
use actix::prelude::*;
use common::AHandler;
use std::collections::{HashSet};
use std::time::Duration;
use tokio::sync::oneshot;
use crate::packet_dispatcher::messages::prune::PruneMessage;

use crate::packet_dispatcher::PacketDispatcherTrait;

pub mod impl_traits;
pub mod messages;
pub mod packets;

const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_secs(10);
const TIME_UNTIL_ERROR: Duration = Duration::from_secs(60);

pub struct DistMutex<P: PacketDispatcherTrait> {
    server_id: ServerId,
    id: ResourceId,
    dispatcher: Addr<P>,

    queue: Vec<(Timestamp, ServerId)>,
    lock_timestamp: Option<Timestamp>,

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
    fn new(server_id: ServerId, id: ResourceId, dispatcher: Addr<P>) -> Self;
}

impl<P: PacketDispatcherTrait> DistMutexTrait for DistMutex<P> {}

impl<P: PacketDispatcherTrait> MutexCreationTrait<P> for DistMutex<P> {
    fn new(server_id: ServerId, id: ResourceId, dispatcher: Addr<P>) -> Self {
        Self {
            server_id,
            id,
            dispatcher,
            lock_timestamp: None,
            ack_received: HashSet::new(),
            ok_received: HashSet::new(),
            all_oks_received_channel: None,
            queue: Vec::new(),
        }
    }
}

impl<P: PacketDispatcherTrait> Actor for DistMutex<P> {
    type Context = Context<Self>;
}

impl<P: PacketDispatcherTrait> DistMutex<P> {
    fn clean_state(&mut self) {
        self.ack_received.clear();
        self.ok_received.clear();
        self.all_oks_received_channel = None;
    }

    fn send_prune(&mut self) {
        let message = PruneMessage {
            older_than: self.lock_timestamp.unwrap(),
        };
        self.dispatcher.do_send(message);
    }
}
