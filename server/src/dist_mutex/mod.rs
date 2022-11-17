use crate::dist_mutex::messages::ack::AckMessage;
use crate::dist_mutex::messages::ok::OkMessage;
use crate::dist_mutex::messages::public::acquire::AcquireMessage;
use crate::dist_mutex::messages::public::release::ReleaseMessage;
use crate::dist_mutex::messages::request::RequestMessage;
use crate::dist_mutex::packets::{ResourceId, Timestamp};
use crate::dist_mutex::server_id::ServerId;
use crate::packet_dispatcher::messages::prune::PruneMessage;
use actix::prelude::*;
use common::AHandler;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::oneshot;

use crate::packet_dispatcher::PacketDispatcherTrait;

pub mod impl_traits;
pub mod messages;
pub mod packets;
pub mod server_id;

#[cfg(not(test))]
const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_secs(10);
#[cfg(test)]
const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_millis(10);

#[cfg(not(test))]
const TIME_UNTIL_ERROR: Duration = Duration::from_secs(60);
#[cfg(test)]
const TIME_UNTIL_ERROR: Duration = Duration::from_millis(10);

pub struct DistMutex<P: Actor> {
    server_id: ServerId,
    id: ResourceId,
    dispatcher: Addr<P>,

    queue: Vec<(Timestamp, ServerId)>,
    lock_timestamp: Option<Timestamp>,

    ack_received: HashSet<ServerId>,
    ok_received: HashSet<ServerId>,
    all_oks_received_channel: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum MutexError {
    Timeout,
    Disconnected,
    Mailbox(String),
}

impl From<MailboxError> for MutexError {
    fn from(e: MailboxError) -> Self {
        MutexError::Mailbox(e.to_string())
    }
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

pub trait MutexCreationTrait<P: Actor> {
    fn new(server_id: ServerId, id: ResourceId, dispatcher: Addr<P>) -> Self;
}

impl<P: PacketDispatcherTrait> DistMutexTrait for DistMutex<P> {}

impl<P: Actor> MutexCreationTrait<P> for DistMutex<P> {
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

impl<P: Actor> Actor for DistMutex<P> {
    type Context = Context<Self>;
}

impl<P: AHandler<PruneMessage>> DistMutex<P> {
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
