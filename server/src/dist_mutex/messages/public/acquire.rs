use actix::fut::LocalBoxActorFuture;
use actix::prelude::*;
use tokio::sync::oneshot;
use tokio::time;

use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::{MutexPacket, RequestPacket};
use crate::dist_mutex::MutexError::Mailbox;
use crate::dist_mutex::{
    DistMutex, MutexError, MutexResult, TIME_UNTIL_DISCONNECT_POLITIC, TIME_UNTIL_ERROR,
};
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::packet::PacketType;
use crate::packet_dispatcher::PacketDispatcherTrait;

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct AcquireMessage;

impl AcquireMessage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AcquireMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: PacketDispatcherTrait> Handler<AcquireMessage> for DistMutex<P> {
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: AcquireMessage, _: &mut Self::Context) -> Self::Result {
        self.clean_state();
        let timestamp = Timestamp::new();

        {
            let packet = MutexPacket::Request(RequestPacket::new(self.id, timestamp));
            self.dispatcher
                .try_send(BroadcastMessage {
                    data: packet.into(),
                    packet_type: PacketType::Mutex,
                })
                .unwrap();
        }
        self.lock_timestamp = Some(timestamp);

        self.queue.push((timestamp, self.server_id));
        self.queue.sort_by_key(|(timestamp, _)| *timestamp);

        let (tx, rx) = oneshot::channel();
        self.all_oks_received_channel = Some(tx);
        let id = self.id;

        let future = async move {
            println!("[Mutex {}] Waiting for all oks", id);
            if time::timeout(TIME_UNTIL_DISCONNECT_POLITIC, rx)
                .await
                .is_err()
            {
                println!("[Mutex {}] Timeout while waiting for oks", id);
                Err(MutexError::Timeout)
            } else {
                println!("[Mutex {}] All oks received", id);
                Ok(())
            }
        }
        .into_actor(self);

        future
            .then(|r, me, _| {
                match r {
                    Ok(()) => {
                        // Lock acquired
                        println!("[Mutex {}] I have the lock", me.id);
                        me.ok_future()
                    }
                    Err(MutexError::Timeout) => {
                        if me.ack_received.is_empty() {
                            // We are disconnected
                            // TODO: Handle this
                            println!("[Mutex {}] We are disconnected", me.id);
                            me.ok_future()
                        } else if me.ok_received == me.ack_received {
                            // There are servers that are disconnected
                            // but we have the lock
                            println!("[Mutex {}] I have the lock", me.id);
                            me.send_prune();
                            me.ok_future()
                        } else {
                            // There is a server that has the lock
                            println!("[Mutex {}] There is a server that has the lock", me.id);
                            me.wait_lock()
                        }
                    }
                    Err(Mailbox(_)) => {
                        panic!("Mailbox error");
                    }
                }
            })
            .boxed_local()
    }
}

impl<P: PacketDispatcherTrait> DistMutex<P> {
    fn ok_future(&mut self) -> LocalBoxActorFuture<DistMutex<P>, Result<(), MutexError>> {
        async { Ok(()) }.into_actor(self).boxed_local()
    }

    fn wait_lock(&mut self) -> LocalBoxActorFuture<DistMutex<P>, Result<(), MutexError>> {
        println!(
            "{} Waiting {} ms for lock",
            self,
            TIME_UNTIL_ERROR.as_millis()
        );
        let (tx, rx) = oneshot::channel();
        self.all_oks_received_channel = Some(tx);
        async move {
            if time::timeout(TIME_UNTIL_ERROR, rx).await.is_err() {
                println!("Timeout while waiting for oks");
                Err(MutexError::Timeout)
            } else {
                Ok(())
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}
