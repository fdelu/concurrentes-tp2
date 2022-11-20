use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::messages::public::commit_request::CommitRequestMessage;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{PacketDispatcherError, PacketDispatcherResult, TransactionId};
use crate::{AcquireMessage, PacketDispatcher};
use actix::prelude::*;
use tracing::{debug, error, info};

#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct BlockPointsMessage {
    pub transaction_id: TransactionId,
    pub client_id: ClientId,
    pub amount: u32,
}

impl Handler<BlockPointsMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, PacketDispatcherResult<()>>;

    fn handle(&mut self, msg: BlockPointsMessage, _ctx: &mut Self::Context) -> Self::Result {
        let transaction = Transaction::Discount {
            id: msg.client_id,
            amount: msg.amount,
        };

        let mutex = self.mutexes.get_mut(&msg.client_id).unwrap();
        let mutex_c = mutex.clone();
        let tp_commit_addr = self.two_phase_commit.clone();

        async move {
            if mutex_c.send(AcquireMessage {}).await.is_err() {
                error!("Error from mutex");
                return Err(PacketDispatcherError::Timeout);
            };

            info!("Acquired mutex for client {}", msg.client_id);
            let mut res = Ok(());
            match tp_commit_addr
                .send(CommitRequestMessage {
                    id: msg.transaction_id,
                    transaction,
                })
                .await
                .unwrap()
            {
                Ok(true) => {
                    debug!("Transaction (Block) successful");
                }
                Ok(false) => {
                    res = Err(PacketDispatcherError::InsufficientPoints);
                }
                Err(e) => {
                    error!("Transaction failed: {:?}", e);
                }
            }
            if mutex_c.send(crate::ReleaseMessage {}).await.is_err() {
                return Err(PacketDispatcherError::Timeout);
            };
            info!("Released mutex for client {}", msg.client_id);
            res
        }
        .into_actor(self)
        .boxed_local()
    }
}
