use actix::prelude::*;
use tokio::sync::oneshot;
use tokio::time;

use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::{
    DistMutex, MutexError, MutexResult, TCPActorTrait, TIME_UNTIL_DISCONNECT_POLITIC,
    TIME_UNTIL_ERROR,
};

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct LockMessage;

impl<T: TCPActorTrait> Handler<LockMessage> for DistMutex<T> {
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: LockMessage, _: &mut Self::Context) -> Self::Result {
        self.clean_state();
        self.lock_timestamp = Some(Timestamp::new());

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
