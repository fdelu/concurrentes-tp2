use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Duration;

use actix::dev::ToEnvelope;
use actix::prelude::*;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time;
use tokio::time::sleep;

use crate::network::{ConnectionlessTCP, SendPacket, SocketError};

mod network;

const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_secs(3);
const TIME_UNTIL_ERROR: Duration = Duration::from_secs(60);

type ResourceId = u32;
type ServerId = SocketAddr;


#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct Lock;

#[derive(Message)]
#[rtype(result = "()")]
pub struct AckReceived {
    pub server_id: ServerId,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OkReceived {
    pub server_id: ServerId,
}

pub struct DistMutex {
    id: ResourceId,
    socket: Addr<ConnectionlessTCP>,
    connected_servers: Vec<SocketAddr>,
    ack_received: HashSet<SocketAddr>,
    ok_received: HashSet<ServerId>,
    sleep_handle: Option<JoinHandle<()>>,
    all_oks_received_channel: Option<oneshot::Sender<()>>,
}

impl Actor for DistMutex {
    type Context = Context<Self>;
}

impl DistMutex {
    fn are_all_ack_received(&self) -> bool {
        self.connected_servers.iter().all(|server_id| self.ack_received.contains(server_id))
    }

    fn are_all_ok_received(&self) -> bool {
        self.connected_servers.iter().all(|server_id| self.ok_received.contains(server_id))
    }
}

impl Handler<Lock> for DistMutex {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: Lock, ctx: &mut Self::Context) -> Self::Result {
        self.ok_received = HashSet::new();
        self.ack_received = HashSet::new();

        self.connected_servers.iter().for_each(|addr| {
            self.socket.do_send(SendPacket {
                to: *addr,
                data: vec![0],
            });
        });
        self.sleep_handle = Some(tokio::spawn(async move {
            sleep(TIME_UNTIL_DISCONNECT_POLITIC).await;
        }));
        let (tx, rx) = oneshot::channel();
        self.all_oks_received_channel = Some(tx);

        Box::pin(async move {
            if let Err(_) = time::timeout(TIME_UNTIL_DISCONNECT_POLITIC, rx).await {}
        }.into_actor(self).map(|a, me, x| {
            if me.are_all_ack_received() {
                if me.are_all_ok_received() {
                    async move {
                        Ok(())
                    }.into_actor(me).boxed_local()
                } else {
                    async move {
                        sleep(TIME_UNTIL_DISCONNECT_POLITIC).await;
                        Ok(())
                    }.into_actor(me).boxed_local()
                }
            } else {
                async move {
                    sleep(TIME_UNTIL_ERROR).await;
                    Err(SocketError::new("Timeout"))
                }.into_actor(me).boxed_local()
            }
        }).map(|_, me, _| {
            Ok(())
        }))
    }
}


impl Handler<AckReceived> for DistMutex {
    type Result = ();

    fn handle(&mut self, msg: AckReceived, ctx: &mut Self::Context) {
        self.ack_received.insert(msg.server_id);
    }
}

impl Handler<OkReceived> for DistMutex {
    type Result = ();

    fn handle(&mut self, msg: OkReceived, ctx: &mut Self::Context) {
        self.ok_received.insert(msg.server_id);
        if self.are_all_ok_received() {
            self.sleep_handle.take().unwrap().abort();
        }
    }
}


fn main() {
    println!("Hello, world!");
}
