use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::{AckPacket, OkPacket, RequestPacket};
use crate::dist_mutex::{DistMutex, ResourceId, ServerId};
use crate::packet_dispatcher::PacketDispatcherTrait;
use actix::prelude::*;
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::PacketType;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RequestMessage {
    from: ServerId,
    timestamp: Timestamp,
}

impl RequestMessage {
    pub fn new(from: ServerId, packet: RequestPacket) -> Self {
        Self {
            from,
            timestamp: packet.timestamp(),
        }
    }
}

fn send_ack<P: PacketDispatcherTrait>(
    dispatcher: &Addr<P>,
    requester: ServerId,
    resource_id: ResourceId,
) {
    let packet = AckPacket::new(resource_id);
    dispatcher
        .try_send(SendMessage {
            to: requester,
            data: packet.into(),
            packet_type: PacketType::Mutex,
        })
        .unwrap();
}

fn send_ok<P: PacketDispatcherTrait>(
    dispatcher: &Addr<P>,
    requester: ServerId,
    resource_id: ResourceId,
) {
    let packet = OkPacket::new(resource_id);
    dispatcher
        .try_send(SendMessage {
            to: requester,
            data: packet.into(),
            packet_type: PacketType::Mutex,
        })
        .unwrap();
}

impl<P: PacketDispatcherTrait> Handler<RequestMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: RequestMessage, _ctx: &mut Self::Context) {
        send_ack(&self.dispatcher, msg.from, self.id);
        if let Some(my_timestamp) = &self.lock_timestamp {
            if my_timestamp > &msg.timestamp {
                println!("{} {:?} has priority over me, sending ok", self, msg.from);
                send_ok(&self.dispatcher, msg.from, self.id);
            } else {
                println!("{} I have priority over {}", self, msg.from);
                // TODO: add to queue
            }
        } else {
            println!(
                "{} I am not waiting for lock, sending ok to {:?}",
                self, msg.from
            );
            send_ok(&self.dispatcher, msg.from, self.id);
        }
    }
}
