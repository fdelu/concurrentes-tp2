use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::messages::public::submit::SubmitMessage;
use crate::two_phase_commit::packets::Transaction;
use crate::{AcquireMessage, PacketDispatcher};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct BlockPointsMessage {
    pub client_id: ClientId,
    pub amount: u32,
}

impl Handler<BlockPointsMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: BlockPointsMessage, _ctx: &mut Self::Context) -> Self::Result {
        let mutex = self.mutexes.get_mut(&msg.client_id).unwrap();
        let mutex_c = mutex.clone();
        let tp_commit_addr = self.two_phase_commit.clone();

        async move {
            mutex_c.send(AcquireMessage {}).await.unwrap();
            println!("Acquired mutex for client {}", msg.client_id);
            tp_commit_addr
                .send(SubmitMessage {
                    transaction: Transaction::Block {
                        id: msg.client_id,
                        amount: msg.amount,
                    },
                })
                .await
                .unwrap();
            println!("Done transaction");
            mutex_c.send(crate::ReleaseMessage {}).await.unwrap();
            println!("Released mutex for client {}", msg.client_id);
        }
        .into_actor(self)
        .boxed_local()
    }
}
