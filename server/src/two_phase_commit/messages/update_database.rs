use crate::dist_mutex::packets::Timestamp;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{TransactionId, TransactionState, TwoPhaseCommit, UserData};
use actix::prelude::*;
use common::packet::UserId;
use std::collections::HashMap;
use tracing::debug;

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateDatabaseMessage {
    pub snapshot_from: Timestamp,
    pub database: HashMap<UserId, UserData>,
    pub logs: HashMap<TransactionId, (TransactionState, Transaction)>,
}

impl<P: Actor> Handler<UpdateDatabaseMessage> for TwoPhaseCommit<P> {
    type Result = ();

    fn handle(&mut self, msg: UpdateDatabaseMessage, ctx: &mut Self::Context) -> Self::Result {
        if msg.snapshot_from > self.database_last_update {
            debug!(
                "Updating database from {} to {}",
                self.database_last_update, msg.snapshot_from
            );
            msg.logs.iter().for_each(|(id, (state, _))| {
                if *state == TransactionState::Prepared {
                    self.set_timeout_for_transaction(*id, ctx);
                }
            });
            self.database = msg.database;
            self.logs = msg.logs;
            self.database_last_update = msg.snapshot_from;
        } else {
            debug!(
                "Ignoring database update ({} >= {})",
                self.database_last_update, msg.snapshot_from
            );
        }
    }
}
