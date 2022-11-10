use actix::prelude::*;

use crate::dist_mutex::packets::MutexPacket;
use crate::dist_mutex::{DistMutexTrait, MutexCreationTrait, TCPActorTrait};
use crate::packet_dispatcher::packet::PacketType;
use crate::packet_dispatcher::PacketDispatcher;

#[derive(Message)]
#[rtype(result = "()")]
pub struct DispatchMessage {
    data: Vec<u8>,
}

impl<D: DistMutexTrait + MutexCreationTrait<Self>, T: TCPActorTrait> Handler<DispatchMessage> for PacketDispatcher<D, T> {
    type Result = ();

    fn handle(&mut self, msg: DispatchMessage, ctx: &mut Self::Context) {
        let mut data = msg.data;
        let packet_type: PacketType = data.remove(0).try_into().unwrap();
        match packet_type {
            PacketType::Mutex => {
                let packet = MutexPacket::try_from(data).unwrap();
                self.handle_mutex(packet, ctx);
            }
            PacketType::Commit => {
                unimplemented!("Commit packet not implemented");
            }
        }
    }
}
