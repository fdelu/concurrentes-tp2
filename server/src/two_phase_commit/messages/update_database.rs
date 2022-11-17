use crate::dist_mutex::packets::Timestamp;
use crate::packet_dispatcher::ClientId;
use crate::two_phase_commit::{ClientData, TwoPhaseCommit};
use actix::prelude::*;
use std::collections::HashMap;

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateDatabaseMessage {
    pub snapshot_from: Timestamp,
    pub database: HashMap<ClientId, ClientData>,
}

impl<P: Actor> Handler<UpdateDatabaseMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: UpdateDatabaseMessage, _ctx: &mut Self::Context) -> Self::Result {
        if msg.snapshot_from > self.database_last_update {
            println!(
                "Updating database from {} to {}",
                self.database_last_update, msg.snapshot_from
            );
            self.database_last_update = msg.snapshot_from;
            self.database = msg.database;
        } else {
            println!(
                "Ignoring database update from {} to {}",
                self.database_last_update, msg.snapshot_from
            );
        }
    }
}
