use crate::dist_mutex::messages::ack::AckMessage;
use crate::dist_mutex::messages::ok::OkMessage;
use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::{AckPacket, OkPacket};
use crate::dist_mutex::{DistMutex, ResourceId, ServerId, TCPActorTrait};
use crate::network::SendPacket;
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct RequestMessage {
    requester: ServerId,
    timestamp: Timestamp,
}

fn send_ack<T: TCPActorTrait>(socket: &Addr<T>, requester: ServerId, resource_id: ResourceId) {
    let packet = AckPacket::new(resource_id, requester);
    socket
        .try_send(SendPacket {
            to: requester.into(),
            data: packet.into(),
        })
        .unwrap();
}

fn send_ok<T: TCPActorTrait>(socket: &Addr<T>, requester: ServerId, resource_id: ResourceId) {
    let packet = OkPacket::new(resource_id, requester);
    socket
        .try_send(SendPacket {
            to: requester.into(),
            data: packet.into(),
        })
        .unwrap();
}

impl<T: TCPActorTrait> Handler<RequestMessage> for DistMutex<T> {
    type Result = ();

    fn handle(&mut self, msg: RequestMessage, _ctx: &mut Self::Context) {
        send_ack(&self.socket, msg.requester, self.id);
        if let Some(my_timestamp) = &self.lock_timestamp {
            if my_timestamp > &msg.timestamp {
                send_ok(&self.socket, msg.requester, self.id);
            }
        } else {
            send_ok(&self.socket, msg.requester, self.id);
        }
    }
}
