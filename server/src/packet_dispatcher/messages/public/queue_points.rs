use actix::prelude::*;
use common::packet::UserId;
use crate::PacketDispatcher;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct QueuePointsMessage {
    pub id: UserId,
    pub amount: u32,
}

impl Handler<QueuePointsMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: QueuePointsMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.points_queue.push(msg);
    }
}