use crate::dist_mutex::packets::Timestamp;
use crate::packet_dispatcher::packet::Packet;
use crate::packet_dispatcher::TransactionId;
use crate::two_phase_commit::PacketDispatcherResult;
use crate::ServerId;
use actix::prelude::*;
use common::packet::UserId;
use common::socket::SocketError;

#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct BlockPointsMessage {
    pub transaction_id: TransactionId,
    pub user_id: UserId,
    pub amount: u32,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct DieMessage;

#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct DiscountMessage {
    pub user_id: UserId,
    pub transaction_id: TransactionId,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct QueuePointsMessage {
    pub id: UserId,
    pub amount: u32,
}

impl QueuePointsMessage {
    pub fn to_add_points_msg(&self) -> AddPointsMessage {
        AddPointsMessage {
            id: self.id,
            amount: self.amount,
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct BroadcastMessage {
    pub packet: Packet,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PruneMessage {
    pub older_than: Timestamp,
}

#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendMessage {
    pub to: ServerId,
    pub packet: Packet,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TryAddPointsMessage;

#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct AddPointsMessage {
    pub id: UserId,
    pub amount: u32,
}
