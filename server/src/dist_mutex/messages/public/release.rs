use actix::prelude::*;

use common::AHandler;
use tracing::error;

use crate::dist_mutex::packets::{MutexPacket, OkPacket};
use crate::dist_mutex::{DistMutex, MutexResult};
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::Packet;

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct ReleaseMessage;

impl<P: AHandler<SendMessage>> Handler<ReleaseMessage> for DistMutex<P> {
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: ReleaseMessage, _: &mut Self::Context) -> Self::Result {
        let dispatcher = self.dispatcher.clone();
        let id = self.id;

        let packet = OkPacket { id };

        let futures: Vec<_> = self
            .queue
            .iter()
            .filter(|(_, id)| *id != self.server_id)
            .map(|(_, server_id)| {
                dispatcher.send(SendMessage {
                    to: *server_id,
                    packet: Packet::Mutex(MutexPacket::Ok(packet)),
                })
            })
            .collect();

        self.queue.clear();
        self.lock_timestamp = None;
        async move {
            for future in futures {
                if future.await.is_err() {
                    error!("[Mutex {}] Error while sending ok to server", id);
                }
            }
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}
