use actix::fut::LocalBoxActorFuture;
use actix::prelude::*;
use tokio::sync::oneshot;
use tokio::time;

use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::{
    DistMutex, MutexError, MutexResult, TIME_UNTIL_DISCONNECT_POLITIC, TIME_UNTIL_ERROR,
};
use crate::network::error::SocketError;
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

        println!("{} Acquiring lock with timestamp {:?}", self, timestamp);
        println!("{} Connected servers: {:?}", self, self.connected_servers);
        self.broadcast_lock_request(timestamp);
        self.lock_timestamp = Some(timestamp);

        let (tx, rx) = oneshot::channel();
        self.all_oks_received_channel = Some(tx);
        let id = self.id;

        let future = async move {
            println!("[Mutex {}] Waiting for all ok's and ack's", id);
            if time::timeout(TIME_UNTIL_DISCONNECT_POLITIC, rx)
                .await
                .is_err()
            {
                println!("[Mutex {}] Timeout while waiting for acks", id);
                Err(MutexError::Timeout)
            } else {
                println!("[Mutex {}] All acks received", id);
                Ok(())
            }
        }
        .into_actor(self);

        future
            .then(|r, me, ctx| {
                match r {
                    Ok(_) => {
                        // Lock acquired
                        if me.are_all_ok_received() {
                            //
                            me.ok_future()
                        } else {
                            // The lock is being used by another server
                            me.wait_lock()
                        }
                    }
                    Err(MutexError::Timeout) => {
                        me.disconnect_servers_with_no_ack();
                        if me.connected_servers.is_empty() {
                            println!("{} I am disconnected", me);
                            // TODO: Disconnect state
                            me.ok_future()
                        } else if me.are_all_ok_received() {
                            me.ok_future()
                        } else {
                            me.wait_lock()
                        }
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
