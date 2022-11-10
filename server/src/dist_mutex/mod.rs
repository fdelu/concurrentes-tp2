use actix::prelude::*;
use common::AHandler;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time;
use crate::dist_mutex::packets::LockRequestPacket;

use crate::network::{ConnectionHandler, SendPacket};

mod packets;

const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_secs(3);
const TIME_UNTIL_ERROR: Duration = Duration::from_secs(60);

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

type ServerId = SocketAddr;

pub enum MutexError {
    Timeout,
}

type MutexResult<T> = Result<T, MutexError>;

pub trait TCPActorTrait: AHandler<SendPacket> {}

impl TCPActorTrait for ConnectionHandler {}

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct LockMessage;

#[derive(Message)]
#[rtype(result = "()")]
pub struct AckReceivedMessage {
    pub server_id: ServerId,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OkReceived {
    pub server_id: ServerId,
}

pub struct DistMutex<T: TCPActorTrait> {
    id: ResourceId,
    socket: Addr<T>,
    connected_servers: Vec<SocketAddr>,
    ack_received: HashSet<SocketAddr>,
    ok_received: HashSet<ServerId>,
    sleep_handle: Option<JoinHandle<()>>,
    all_oks_received_channel: Option<oneshot::Sender<()>>,
}

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
        let lock_request = LockRequestPacket::new(self.id);

        self.connected_servers.iter().for_each(|addr| {
            self.socket.do_send(SendPacket {
                to: *addr,
                data: lock_request.into(),
            });
        });
    }
}

impl<T: TCPActorTrait> Handler<LockMessage> for DistMutex<T> {
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: LockMessage, _: &mut Self::Context) -> Self::Result {
        self.clean_state();

        self.broadcast_lock_request();

        let (tx, rx) = oneshot::channel();
        self.all_oks_received_channel = Some(tx);

        let future = async move {
            if time::timeout(TIME_UNTIL_DISCONNECT_POLITIC, rx)
                .await
                .is_err()
            {
                println!("Timeout while waiting for acks");
                Err(MutexError::Timeout)
            } else {
                Ok(())
            }
        }
        .into_actor(self);

        future
            .then(|r, me, _| {
                match r {
                    Ok(_) => {
                        // Lock acquired
                        if me.are_all_ok_received() {
                            //
                            async { Ok(()) }.into_actor(me).boxed_local()
                        } else {
                            // The lock is being used by another server
                            let (tx, rx) = oneshot::channel();
                            me.all_oks_received_channel = Some(tx);
                            async move {
                                if time::timeout(TIME_UNTIL_ERROR, rx).await.is_err() {
                                    println!("Timeout while waiting for oks");
                                    Err(MutexError::Timeout)
                                } else {
                                    Ok(())
                                }
                            }
                            .into_actor(me)
                            .boxed_local()
                        }
                    }
                    Err(MutexError::Timeout) => {
                        // TODO: Disconnect politic
                        async { Ok(()) }.into_actor(me).boxed_local()
                    }
                }
            })
            .boxed_local()
    }
}

impl<T: TCPActorTrait> Handler<AckReceivedMessage> for DistMutex<T> {
    type Result = ();

    fn handle(&mut self, msg: AckReceivedMessage, _ctx: &mut Self::Context) {
        self.ack_received.insert(msg.server_id);
    }
}

impl<T: TCPActorTrait> Handler<OkReceived> for DistMutex<T> {
    type Result = ();

    fn handle(&mut self, msg: OkReceived, _ctx: &mut Self::Context) {
        self.ok_received.insert(msg.server_id);
        if self.are_all_ok_received() {
            self.sleep_handle.take().unwrap().abort();
        }
    }
}

