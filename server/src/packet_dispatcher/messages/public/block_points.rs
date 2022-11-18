use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::messages::public::commit_request::CommitRequestMessage;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{PacketDispatcherError, PacketDispatcherResult, TransactionId};
use crate::{AcquireMessage, PacketDispatcher};
use actix::prelude::*;

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
                println!("Error from mutex");
                return Err(PacketDispatcherError::Timeout);
            };

            println!("Acquired mutex for client {}", msg.client_id);
            match tp_commit_addr
                .send(CommitRequestMessage {
                    id: msg.transaction_id,
                    transaction,
                })
                .await
                .unwrap()
            {
                Ok(_) => {
                    println!("Transaction (Block) successful");
                }
                Err(e) => {
                    println!("Transaction failed: {:?}", e);
                }
            }
            if mutex_c.send(crate::ReleaseMessage {}).await.is_err() {
                return Err(PacketDispatcherError::Timeout);
            };
            println!("Released mutex for client {}", msg.client_id);
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}
