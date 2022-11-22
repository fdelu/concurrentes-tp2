use crate::dist_mutex::messages::DoWithLock;
use crate::dist_mutex::packets::get_timestamp;
use crate::dist_mutex::DistMutex;
use crate::packet_dispatcher::messages::{
    AddPointsMessage, BlockPointsMessage, BroadcastMessage, DieMessage, DiscountMessage,
    PruneMessage, QueuePointsMessage, SendMessage, TryAddPointsMessage,
};
use crate::packet_dispatcher::packet::Packet;
use crate::packet_dispatcher::TransactionId;
use crate::two_phase_commit::messages::{
    CommitCompleteMessage, CommitRequestMessage, ForwardDatabaseMessage,
};

use super::error::{PacketDispatcherError, PacketDispatcherResult};
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::TwoPhaseCommit;
use crate::{Listen, PacketDispatcher, ServerId};
use actix::prelude::*;
use common::socket::{ReceivedPacket, SocketError};
use std::collections::HashSet;
use std::time::Duration;
use tracing::{debug, error, info, trace, warn};

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
            let action = move || {
                tp_commit_addr.send(CommitRequestMessage {
                    id: msg.transaction_id,
                    transaction,
                })
            };
            match mutex_c.send(DoWithLock { action }).await {
                Ok(Ok(Ok(Ok(true)))) => {
                    info!("Transaction {} succeeded", msg.transaction_id);
                    Ok(())
                }
                Ok(Ok(Ok(Ok(false)))) => {
                    error!(
                        "Transaction {} failed because user has insufficient points",
                        msg.transaction_id
                    );
                    Err(PacketDispatcherError::InsufficientPoints)
                }
                _ => {
                    error!("Transaction {} failed", msg.transaction_id);
                    Err(PacketDispatcherError::Other)
                }
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<DieMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, _msg: DieMessage, _: &mut Self::Context) -> Self::Result {
        debug!("[PacketDispatcher] Received DieMessage");
        self.mutexes.values().for_each(|mutex| {
            mutex.do_send(DieMessage);
        });
    }
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
            let action = move || {
                tp_commit_addr.send(CommitCompleteMessage {
                    id: msg.transaction_id,
                    connected_servers,
                })
            };
            match mutex.send(DoWithLock { action }).await {
                Ok(Ok(Ok(Ok(())))) => {
                    debug!("Transaction {} succeeded", msg.transaction_id);
                    Ok(())
                }
                _ => {
                    error!("Transaction {} failed", msg.transaction_id);
                    Err(PacketDispatcherError::DiscountFailed)
                }
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<QueuePointsMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: QueuePointsMessage, _ctx: &mut Self::Context) -> Self::Result {
        debug!(
            "[PacketDispatcher] Received QueuePointsMessage for {} of {} points",
            msg.id, msg.amount
        );
        self.points_queue.push(msg);
    }
}

impl Handler<BroadcastMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: BroadcastMessage, _ctx: &mut Self::Context) -> Self::Result {
        debug!(
            "Broadcasting to {} servers",
            self.get_connected_servers().len()
        );
        trace!("Connected servers: {:?}", self.get_connected_servers());

        let futures: Vec<_> = self
            .get_connected_servers()
            .iter()
            .map(|server_id| {
                debug!("Sending to {}", server_id);
                self.send_data(*server_id, msg.packet.clone())
            })
            .collect();

        async move {
            for future in futures {
                future.await??;
            }
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<ReceivedPacket<Packet>> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: ReceivedPacket<Packet>, ctx: &mut Self::Context) {
        let origin = msg.addr.into();
        let packet: Packet = msg.data;

        trace!("Received a packet from {}: {:?}", origin, packet);

        self.servers_last_seen.insert(origin, Some(get_timestamp()));

        match packet {
            Packet::Mutex(packet) => {
                self.handle_mutex(origin, packet, ctx);
            }
            Packet::Commit(packet) => {
                self.handle_commit(origin, packet, ctx);
            }
            Packet::SyncRequest(_) => {
                info!("Received sync request from {}", origin);
                self.two_phase_commit
                    .do_send(ForwardDatabaseMessage { to: origin });
            }
            Packet::SyncResponse(packet) => {
                info!("Received sync response from {}", origin);
                self.two_phase_commit.do_send(packet.to_update_db_msg());
            }
        }
    }
}

impl Handler<Listen> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: Listen, _ctx: &mut Self::Context) -> Self::Result {
        let socket_actor_addr = self.socket.clone();

        async move {
            match socket_actor_addr.send(msg).await {
                Ok(_) => Ok(()),
                Err(e) => Err(SocketError::from(e)),
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<PruneMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: PruneMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.servers_last_seen = self
            .servers_last_seen
            .iter()
            .map(|(server_id, timestamp)| {
                if let Some(timestamp) = timestamp {
                    if *timestamp < msg.older_than {
                        (*server_id, None)
                    } else {
                        (*server_id, Some(*timestamp))
                    }
                } else {
                    (*server_id, None)
                }
            })
            .collect();
    }
}

impl Handler<SendMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SendMessage, _ctx: &mut Self::Context) -> Self::Result {
        let id: ServerId = msg.to;
        let fut = self.send_data(id, msg.packet.clone());

        async move {
            match fut.await? {
                Ok(()) => Ok(()),
                Err(e) => {
                    error!("Error sending packet to {}: {}", msg.to, e);
                    Err(e)
                }
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<TryAddPointsMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: TryAddPointsMessage, ctx: &mut Self::Context) -> Self::Result {
        trace!("Trying to add points {:?}", self.points_queue);

        let addr = ctx.address();
        let points_list = self.points_queue.clone();
        self.points_queue = vec![];

        async move {
            let mut not_added_points = vec![];
            for points in points_list {
                let result = addr.send(points.to_add_points_msg()).await;
                if let Ok(Ok(())) = result {
                    debug!("Added points to user {}", points.id);
                } else {
                    warn!("Failed to add points to user {}", points.id);
                    debug!("Error: {:?}", result);
                    not_added_points.push(points.clone());
                }
            }
            not_added_points
        }
        .into_actor(self)
        .then(move |not_added_points, me, ctx| {
            let next_attempt_duration = Duration::from_millis(me.config.add_points_interval_ms);
            me.points_queue.extend(not_added_points);
            ctx.notify_later(TryAddPointsMessage, next_attempt_duration);
            async {}.into_actor(me)
        })
        .boxed_local()
    }
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
            make_add_points_commit(
                transaction,
                mutex.clone(),
                tp_commit_addr.clone(),
                connected_servers,
                transaction_id,
            )
            .await?;
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}

async fn make_add_points_commit(
    transaction: Transaction,
    mutex: Addr<DistMutex<PacketDispatcher>>,
    tp_commit_addr: Addr<TwoPhaseCommit<PacketDispatcher>>,
    connected_servers: HashSet<ServerId>,
    transaction_id: TransactionId,
) -> Result<(), PacketDispatcherError> {
    let action = move || async move {
        info!("Requesting commit to add points tx. id {}", transaction_id);
        let can_add = tp_commit_addr
            .send(CommitRequestMessage {
                id: transaction_id,
                transaction,
            })
            .await??;
        if !can_add
            || tp_commit_addr
                .send(CommitCompleteMessage {
                    id: transaction_id,
                    connected_servers,
                })
                .await?
                .is_err()
        {
            return Err(PacketDispatcherError::IncreaseFailed);
        }
        info!("Add points commit for tx. id {} done", transaction_id);
        Ok(())
    };
    match mutex.send(DoWithLock { action }).await? {
        Err(_) => Err(PacketDispatcherError::IncreaseFailed),
        Ok(result) => result,
    }
}
