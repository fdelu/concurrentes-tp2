use actix::prelude::*;

use crate::dist_mutex::packets::get_timestamp;
use crate::dist_mutex::server_id::ServerId;
use crate::network::ReceivedPacket;
use crate::packet_dispatcher::packet::Packet;
use crate::packet_dispatcher::PacketDispatcher;

impl Handler<ReceivedPacket<Packet>> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: ReceivedPacket<Packet>, ctx: &mut Self::Context) {
        let origin_addr = msg.addr;

        let packet: Packet = msg.data;

        self.servers_last_seen
            .insert(origin_addr.into(), Some(get_timestamp()));

        match packet {
            Packet::Mutex(packet) => {
                self.handle_mutex(origin_addr.into(), packet, ctx);
            }
            Packet::Commit(packet) => {
                self.handle_commit(origin_addr.into(), packet, ctx);
            }
            Packet::SyncRequest(packet) => {
                println!("Received sync request from {}", ServerId::from(msg.addr));
                self.handle_sync_request(msg.addr.into(), packet, ctx);
            }
            Packet::SyncResponse(packet) => {
                println!("Received sync response from {}", ServerId::from(msg.addr));
                self.handle_sync_response(msg.addr.into(), packet, ctx);
            }
        }
    }
}
