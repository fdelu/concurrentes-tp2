use crate::{PacketDispatcher, ServerId};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddServerMessage {
    server_id: ServerId,
}

impl AddServerMessage {
    pub fn new(server_id: ServerId) -> Self {
        Self { server_id }
    }
}

impl Handler<AddServerMessage> for PacketDispatcher {
    type Result = ();

    fn handle(&mut self, msg: AddServerMessage, _ctx: &mut Self::Context) -> Self::Result {
        println!("Adding server {}", msg.server_id);
        self.connected_servers.insert(msg.server_id);
    }
}
