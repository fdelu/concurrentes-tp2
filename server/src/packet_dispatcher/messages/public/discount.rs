use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::messages::public::commit_complete::CommitCompleteMessage;
use crate::two_phase_commit::{PacketDispatcherError, PacketDispatcherResult, TransactionId};
use crate::PacketDispatcher;
use actix::prelude::*;
use tracing::{debug, error};

#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct DiscountMessage {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
}

impl Handler<DiscountMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, PacketDispatcherResult<()>>;

    fn handle(&mut self, msg: DiscountMessage, _ctx: &mut Self::Context) -> Self::Result {
        let mutex = self.mutexes.get_mut(&msg.client_id).unwrap();
        let mutex_c_1 = mutex.clone();
        let mutex_c_2 = mutex.clone();
        let tp_commit_addr = self.two_phase_commit.clone();

        async move {
            if mutex_c_1.send(crate::AcquireMessage {}).await.is_err() {
                error!("Error from mutex");
                Err(PacketDispatcherError::Timeout)
            } else {
                debug!("Acquired mutex for client {}", msg.client_id);
                Ok(())
            }
        }
        .into_actor(self)
        .and_then(move |_, me, _| {
            let f = tp_commit_addr.send(CommitCompleteMessage {
                id: msg.transaction_id,
                connected_servers: me.get_connected_servers(),
            });
            async move {
                if f.await.is_err() {
                    Err(PacketDispatcherError::Timeout)
                } else {
                    Ok(())
                }
            }
            .into_actor(me)
            .boxed_local()
        })
        .then(move |_, me, _| {
            async move {
                if mutex_c_2.send(crate::ReleaseMessage {}).await.is_err() {
                    return Err(PacketDispatcherError::Timeout);
                };
                debug!("Released mutex for client {}", msg.client_id);
                Ok(())
            }
            .into_actor(me)
            .boxed_local()
        })
        .boxed_local()
    }
}
