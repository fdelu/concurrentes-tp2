use crate::two_phase_commit::messages::public::commit_request::CommitRequestMessage;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{PacketDispatcherError, PacketDispatcherResult, TransactionId};
use crate::{AcquireMessage, PacketDispatcher};
use actix::prelude::*;
use tracing::{debug, error, info};
use common::packet::UserId;
use crate::dist_mutex::messages::public::do_with_lock::DoWithLock;

#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct BlockPointsMessage {
    pub transaction_id: TransactionId,
    pub user_id: UserId,
    pub amount: u32,
}

impl Handler<BlockPointsMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, PacketDispatcherResult<()>>;

    fn handle(&mut self, msg: BlockPointsMessage, ctx: &mut Self::Context) -> Self::Result {
        let transaction = Transaction::Discount {
            id: msg.user_id,
            amount: msg.amount,
        };

        let mutex = self.get_or_create_mutex(ctx, msg.user_id);
        let mutex_c = mutex.clone();
        let tp_commit_addr = self.two_phase_commit.clone();

        async move {
            let action  = move || {
                tp_commit_addr
                .send(CommitRequestMessage {
                    id: msg.transaction_id,
                    transaction,
                })
            };
            match mutex_c.send(DoWithLock { action }).await
            {
                Ok(Ok(Ok(Ok(true)))) => {
                    info!("Transaction {} succeeded", msg.transaction_id);
                    Ok(())
                },
                Ok(Ok(Ok(Ok(false)))) =>  {
                    error!("Transaction {} failed because user has insufficient points", msg.transaction_id);
                    Err(PacketDispatcherError::InsufficientPoints)
                },
                _ => {
                    error!("Transaction {} failed", msg.transaction_id);
                    Err(PacketDispatcherError::Other)
                },
            }
        }.into_actor(self).boxed_local()
    }
}
