use actix::prelude::*;
use tracing::{info, trace};

use crate::dist_mutex::packets::get_timestamp;
use crate::network::ReceivedPacket;
use crate::packet_dispatcher::packet::Packet;
use crate::packet_dispatcher::PacketDispatcher;
use crate::two_phase_commit::messages::forward_database::ForwardDatabaseMessage;

impl Handler<ReceivedPacket<Packet>> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: ReceivedPacket<Packet>, ctx: &mut Self::Context) {
        let origin = msg.addr.into();
        let packet: Packet = msg.data;

        trace!("Received a packet from {}: {:?}", origin, packet);

        self.servers_last_seen.insert(origin, Some(get_timestamp()));

        match packet {
            Packet::Mutex(packet) => {
                self.handle_mutex(origin, packet, ctx);
            }
            Packet::Commit(packet) => {
                self.handle_commit(origin, packet, ctx);
            }
            Packet::SyncRequest(_) => {
                info!("Received sync request from {}", origin);
                self.two_phase_commit
                    .do_send(ForwardDatabaseMessage { to: origin });
            }
            Packet::SyncResponse(packet) => {
                info!("Received sync response from {}", origin);
                self.two_phase_commit.do_send(packet.to_update_db_msg());
            }
        }
    }
}
