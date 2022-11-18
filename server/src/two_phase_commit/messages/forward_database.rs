use crate::packet_dispatcher::messages::send::SendMessage;
use crate::two_phase_commit::TwoPhaseCommit;
use crate::ServerId;
use actix::prelude::*;

use crate::dist_mutex::packets::get_timestamp;
use crate::packet_dispatcher::packet::{Packet, SyncResponsePacket};
use common::AHandler;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ForwardDatabaseMessage {
    pub to: ServerId,
}

impl<P: AHandler<SendMessage>> Handler<ForwardDatabaseMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: ForwardDatabaseMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.dispatcher.do_send(SendMessage {
            to: msg.to,
            packet: Packet::SyncResponse(SyncResponsePacket {
                snapshot_from: self.database_last_update,
                database: self.database.clone(),
            }),
        });
    }
}
