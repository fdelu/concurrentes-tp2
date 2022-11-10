use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::{AckPacket, MutexPacket, OkPacket, RequestPacket};
use crate::dist_mutex::{DistMutex, ResourceId, ServerId};
use crate::packet_dispatcher::messages::send_from_mutex::SendFromMutexMessage;
use crate::packet_dispatcher::PacketDispatcherTrait;
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct RequestMessage {
    requester: ServerId,
    timestamp: Timestamp,
}

impl RequestMessage {
    pub fn new(packet: RequestPacket) -> Self {
        Self {
            requester: packet.requester(),
            timestamp: packet.timestamp(),
        }
    }
}

fn send_ack<P: PacketDispatcherTrait>(
    dispatcher: &Addr<P>,
    requester: ServerId,
    resource_id: ResourceId,
) {
    let packet = AckPacket::new(resource_id, requester);
    dispatcher
        .try_send(SendFromMutexMessage::new(
            MutexPacket::Ack(packet),
            requester,
        ))
        .unwrap();
}

fn send_ok<P: PacketDispatcherTrait>(
    dispatcher: &Addr<P>,
    requester: ServerId,
    resource_id: ResourceId,
) {
    let packet = OkPacket::new(resource_id, requester);
    dispatcher
        .try_send(SendFromMutexMessage::new(
            MutexPacket::Ok(packet),
            requester,
        ))
        .unwrap();
}

impl<P: PacketDispatcherTrait> Handler<RequestMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: RequestMessage, _ctx: &mut Self::Context) {
        send_ack(&self.dispatcher, msg.requester, self.id);
        if let Some(my_timestamp) = &self.lock_timestamp {
            if my_timestamp > &msg.timestamp {
                send_ok(&self.dispatcher, msg.requester, self.id);
            }
        } else {
            send_ok(&self.dispatcher, msg.requester, self.id);
        }
    }
}
