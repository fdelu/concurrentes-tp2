use actix::prelude::*;
use crate::PacketDispatcher;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct AddPointsMessage {
    pub id: u32,
    pub amount: u32,
}

impl Handler<AddPointsMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, Result<(), String>>;

    fn handle(&mut self, msg: AddPointsMessage, _ctx: &mut Self::Context) -> Self::Result {
        async {
            Ok(())
        }.into_actor(self).boxed_local()
    }
}