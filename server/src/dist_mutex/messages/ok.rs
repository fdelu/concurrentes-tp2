use actix::prelude::*;
use crate::dist_mutex::{DistMutex, ServerId, TCPActorTrait};

#[derive(Message)]
#[rtype(result = "()")]
pub struct OkMessage {
    pub server_id: ServerId,
}

impl<T: TCPActorTrait> Handler<OkMessage> for DistMutex<T> {
    type Result = ();

    fn handle(&mut self, msg: OkMessage, _ctx: &mut Self::Context) {
        self.ok_received.insert(msg.server_id);
        if self.are_all_ok_received() {
            self.sleep_handle.take().unwrap().abort();
        }
    }
}