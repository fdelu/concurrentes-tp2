use actix::prelude::*;

use crate::dist_mutex::packets::OkPacket;
use crate::dist_mutex::{DistMutex, MutexResult};
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::PacketType;

use common::AHandler;

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct ReleaseMessage;

impl<P: AHandler<SendMessage>> Handler<ReleaseMessage> for DistMutex<P> {
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: ReleaseMessage, _: &mut Self::Context) -> Self::Result {
        let dispatcher = self.dispatcher.clone();
        let id = self.id;

        let packet = OkPacket::new(id);
        let data: Vec<u8> = packet.into();

        let futures: Vec<_> = self
            .queue
            .iter()
            .filter(|(_, id)| *id != self.server_id)
            .map(|(_, server_id)| {
                dispatcher.send(SendMessage {
                    data: data.clone(),
                    packet_type: PacketType::Mutex,
                    to: *server_id,
                })
            })
            .collect();

        self.queue.clear();
        self.lock_timestamp = None;
        async move {
            for future in futures {
                if future.await.is_err() {
                    println!("[Mutex {}] Error while sending ok to server", id);
                }
            }
            Ok(())
        }
        .into_actor(self)
        .boxed_local()
    }
}
