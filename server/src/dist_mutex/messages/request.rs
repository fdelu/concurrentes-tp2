use actix::prelude::*;

use common::AHandler;
use tracing::debug;

use crate::dist_mutex::packets::{AckPacket, MutexPacket, OkPacket, RequestPacket, Timestamp};
use crate::dist_mutex::{DistMutex, ResourceId, ServerId};
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::Packet;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RequestMessage {
    pub from: ServerId,
    pub timestamp: Timestamp,
}

impl RequestMessage {
    pub fn new(from: ServerId, packet: RequestPacket) -> Self {
        Self {
            from,
            timestamp: packet.timestamp,
        }
    }
}

fn send_ack<P: AHandler<SendMessage>>(
    dispatcher: &Addr<P>,
    requester: ServerId,
    resource_id: ResourceId,
) {
    let packet = AckPacket { id: resource_id };
    dispatcher
        .try_send(SendMessage {
            to: requester,
            packet: Packet::Mutex(MutexPacket::Ack(packet)),
        })
        .unwrap();
}

fn send_ok<P: AHandler<SendMessage>>(
    dispatcher: &Addr<P>,
    requester: ServerId,
    resource_id: ResourceId,
) {
    let packet = OkPacket { id: resource_id };
    dispatcher
        .try_send(SendMessage {
            to: requester,
            packet: Packet::Mutex(MutexPacket::Ok(packet)),
        })
        .unwrap();
}

impl<P: AHandler<SendMessage>> Handler<RequestMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: RequestMessage, _ctx: &mut Self::Context) {
        send_ack(&self.dispatcher, msg.from, self.id);

        if let Some(my_timestamp) = &self.lock_timestamp {
            if my_timestamp > &msg.timestamp {
                debug!("{} {:?} has priority over me, sending ok", self, msg.from);
                send_ok(&self.dispatcher, msg.from, self.id);
            } else {
                debug!(
                    "{} I have priority over {} (my timestamp {} - other timestamp: {})",
                    self, msg.from, my_timestamp, msg.timestamp
                );
                self.queue.push((msg.timestamp, msg.from));
            }
        } else {
            debug!(
                "{} I am not waiting for lock, sending ok to {}",
                self, msg.from
            );
            send_ok(&self.dispatcher, msg.from, self.id);
        }
    }
}
