use actix::prelude::*;
use crate::dist_mutex::messages::Timestamp;
use crate::PacketDispatcher;

#[derive(Message)]
#[rtype(result = "()")]
pub struct PruneMessage {
    pub older_than: Timestamp,
}

impl Handler<PruneMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: PruneMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.servers_last_seen = self.servers_last_seen
            .iter()
            .map(|(server_id, timestamp)| {
                if let Some(timestamp) = timestamp {
                    if *timestamp < msg.older_than {
                        (*server_id, None)
                    } else {
                        (*server_id, Some(*timestamp))
                    }
                } else {
                    (*server_id, None)
                }
            })
            .collect();
    }
}



