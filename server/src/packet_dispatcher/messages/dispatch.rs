use actix::prelude::*;

use crate::dist_mutex::packets::MutexPacket;
use crate::dist_mutex::MutexCreationTrait;
use crate::network::ReceivedPacket;
use crate::packet_dispatcher::packet::PacketType;
use crate::packet_dispatcher::PacketDispatcher;

impl Handler<ReceivedPacket> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: ReceivedPacket, ctx: &mut Self::Context) {
        let mut data = msg.data;
        let packet_type: PacketType = data.remove(0).try_into().unwrap();

        match packet_type {
            PacketType::Mutex => {
                let packet = MutexPacket::try_from(data).unwrap();
                self.handle_mutex(msg.addr.into(), packet, ctx);
            }
            PacketType::Commit => {
                unimplemented!("Commit packet not implemented");
            }
        }
    }
}
