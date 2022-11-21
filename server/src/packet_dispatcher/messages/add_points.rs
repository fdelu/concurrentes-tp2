use crate::dist_mutex::messages::DoWithLock;
use crate::dist_mutex::DistMutex;
use crate::packet_dispatcher::TransactionId;
use crate::two_phase_commit::messages::CommitCompleteMessage;
use crate::two_phase_commit::messages::CommitRequestMessage;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{PacketDispatcherError, TwoPhaseCommit};
use crate::{PacketDispatcher, ServerId};
use actix::prelude::*;
use common::packet::UserId;
use std::collections::HashSet;
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
        let new_transaction_value = self.points_ids_counter;
        let connected_servers = self.get_connected_servers();

        self.points_ids_counter += 1;
        let server_id = self.server_id;
        let transaction_id = TransactionId::Add(server_id, new_transaction_value);

        async move {
            send_commit_request(
                transaction,
                mutex.clone(),
                tp_commit_addr.clone(),
                transaction_id,
            )
            .await?;
            send_commit_complete(mutex, tp_commit_addr, connected_servers, transaction_id).await?;
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}

async fn send_commit_request(
    transaction: Transaction,
    mutex: Addr<DistMutex<PacketDispatcher>>,
    tp_commit_addr: Addr<TwoPhaseCommit<PacketDispatcher>>,
    transaction_id: TransactionId,
) -> Result<(), PacketDispatcherError> {
    let action = move || {
        tp_commit_addr.send(CommitRequestMessage {
            id: transaction_id,
            transaction,
        })
    };
    let r = mutex.send(DoWithLock { action }).await;
    match r {
        Ok(Ok(Ok(Ok(true)))) => Ok(()),
        _ => Err(PacketDispatcherError::IncreaseFailed),
    }
}

async fn send_commit_complete(
    mutex: Addr<DistMutex<PacketDispatcher>>,
    tp_commit_addr: Addr<TwoPhaseCommit<PacketDispatcher>>,
    connected_servers: HashSet<ServerId>,
    transaction_id: TransactionId,
) -> Result<(), PacketDispatcherError> {
    let action = move || {
        tp_commit_addr.send(CommitCompleteMessage {
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
}
