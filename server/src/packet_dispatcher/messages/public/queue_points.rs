use crate::PacketDispatcher;
use actix::prelude::*;
use common::packet::UserId;
use tracing::debug;

#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct QueuePointsMessage {
    pub id: UserId,
    pub amount: u32,
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
