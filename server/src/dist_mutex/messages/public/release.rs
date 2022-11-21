use actix::prelude::*;

use common::AHandler;

use crate::dist_mutex::packets::{MutexPacket, OkPacket};
use crate::dist_mutex::DistMutex;
use crate::packet_dispatcher::messages::send::SendMessage;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReleaseMessage;

impl<P: AHandler<SendMessage>> Handler<ReleaseMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, _: ReleaseMessage, _: &mut Self::Context) -> Self::Result {
        let id = self.id;

        let ok = OkPacket { id };

        for (_, server) in &self.queue {
            if *server != self.server_id {
                self.do_send(*server, MutexPacket::Ok(ok));
            }
        }
        self.clean_state();
    }
}
