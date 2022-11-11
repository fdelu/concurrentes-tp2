use crate::dist_mutex::packets::MutexPacket;
use crate::dist_mutex::{DistMutexTrait, ServerId};
use crate::network::SendPacket;
use crate::packet_dispatcher::packet::PacketType;
use crate::packet_dispatcher::{PacketDispatcher, TCPActorTrait};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendFromMutexMessage {
    packet: MutexPacket,
    to: ServerId,
}

impl SendFromMutexMessage {
    pub fn new(packet: MutexPacket, to: ServerId) -> Self {
        Self { packet, to }
    }
}

impl Handler<SendFromMutexMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: SendFromMutexMessage, _ctx: &mut Self::Context) {
        let mut data: Vec<u8> = msg.packet.into();
        data.insert(0, PacketType::Mutex as u8);
        self.socket
            .try_send(SendPacket {
                data,
                to: msg.to.into(),
            })
            .unwrap();
    }
}
