use crate::dist_mutex::messages::public::do_with_lock::DoWithLock;
use crate::packet_dispatcher::TransactionId;
use crate::two_phase_commit::messages::public::commit_complete::CommitCompleteMessage;
use crate::two_phase_commit::messages::public::commit_request::CommitRequestMessage;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::PacketDispatcherError;
use crate::PacketDispatcher;
use actix::prelude::*;
use common::packet::UserId;
use tracing::{debug, error};

#[derive(Message)]
#[rtype(result = "Result<(), PacketDispatcherError>")]
pub struct AddPointsMessage {
    pub id: UserId,
    pub amount: u32,
}

impl Handler<AddPointsMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), PacketDispatcherError>>;

    fn handle(&mut self, msg: AddPointsMessage, ctx: &mut Self::Context) -> Self::Result {
        let transaction = Transaction::Increase {
            id: msg.id,
            amount: msg.amount,
        };

        let mutex = self.get_or_create_mutex(ctx, msg.id).clone();
        let tp_commit_addr = self.two_phase_commit.clone();
        let tp_commit_addr_c = tp_commit_addr.clone();
        let new_transaction_value = self.points_ids_counter;
        let connected_servers = self.get_connected_servers();

        self.points_ids_counter += 1;
        let server_id = self.server_id;
        let transaction_id = TransactionId::Add(server_id, new_transaction_value);

        async move {
            let action = move || {
                tp_commit_addr.send(CommitRequestMessage {
                    id: transaction_id,
                    transaction,
                })
            };
            if let Ok(Ok(Ok(Ok(true)))) = mutex.send(DoWithLock { action }).await {
                let action = move || {
                    tp_commit_addr_c.send(CommitCompleteMessage {
                        id: transaction_id,
                        connected_servers,
                    })
                };
                match mutex.send(DoWithLock { action }).await {
                    Ok(Ok(Ok(()))) => {
                        debug!("Transaction {} succeeded", transaction_id);
                        Ok(())
                    }
                    _ => {
                        error!("Transaction {} failed", transaction_id);
                        Err(PacketDispatcherError::IncreaseFailed)
                    }
                }
            } else {
                error!("Transaction {} failed", transaction_id);
                Err(PacketDispatcherError::IncreaseFailed)
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}
