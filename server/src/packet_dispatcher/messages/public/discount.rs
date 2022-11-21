use crate::two_phase_commit::messages::public::commit_complete::CommitCompleteMessage;
use crate::two_phase_commit::{PacketDispatcherError, PacketDispatcherResult};
use crate::PacketDispatcher;
use actix::prelude::*;
use tracing::{debug, error};
use common::packet::UserId;
use crate::dist_mutex::messages::public::do_with_lock::DoWithLock;
use crate::packet_dispatcher::TransactionId;

#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct DiscountMessage {
    pub user_id: UserId,
    pub transaction_id: TransactionId,
}

impl Handler<DiscountMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, PacketDispatcherResult<()>>;

    fn handle(&mut self, msg: DiscountMessage, ctx: &mut Self::Context) -> Self::Result {
        let mutex = self.get_or_create_mutex(ctx, msg.user_id).clone();
        let tp_commit_addr = self.two_phase_commit.clone();
        let connected_servers = self.get_connected_servers();

        async move {
            // Esto será ejecutado cuando el servidor tenga el lock
            // del usuario
            let action  = move || {
                tp_commit_addr
                .send(CommitCompleteMessage {
                    id: msg.transaction_id,
                    connected_servers,
                })
            };
            match mutex.send(DoWithLock { action }).await {
                Ok(Ok(Ok(()))) => {
                    debug!("Transaction {} succeeded", msg.transaction_id);
                    Ok(())
                }
                _ => {
                    error!("Transaction {} failed", msg.transaction_id);
                    Err(PacketDispatcherError::DiscountFailed)
                },
            }
        }.into_actor(self).boxed_local()
    }
}
