use actix::prelude::*;
use crate::packet_dispatcher::ClientId;
use crate::PacketDispatcher;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{PacketDispatcherError, PacketDispatcherResult, TransactionId};
use crate::two_phase_commit::messages::public::submit::SubmitMessage;

#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct DiscountMessage {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
}

impl Handler<DiscountMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, PacketDispatcherResult<()>>;

    fn handle(&mut self, msg: DiscountMessage, ctx: &mut Self::Context) -> Self::Result {
        let transaction = Transaction::Discount {
            id: msg.client_id,
            associated_to: msg.transaction_id,
        };

        let mutex = self.mutexes.get_mut(&msg.client_id).unwrap();
        let mutex_c = mutex.clone();
        let tp_commit_addr = self.two_phase_commit.clone();

        async move {
            if mutex_c.send(crate::AcquireMessage {}).await.is_err() {
                println!("Error from mutex");
                return Err(PacketDispatcherError::Timeout);
            };

            println!("Acquired mutex for client {}", msg.client_id);
            match tp_commit_addr
                .send(SubmitMessage {
                    transaction
                })
                .await.unwrap() {
                    Ok(_) => {
                        println!("Transaction (Discount) successful");
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