use crate::packet_dispatcher::messages::add_server::AddServerMessage;
use crate::{DistMutex, MutexCreationTrait, PacketDispatcher, ResourceId};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "Addr<DistMutex<PacketDispatcher>>")]
pub struct AddMutexMessage {
    id: ResourceId,
}

impl AddMutexMessage {
    pub fn new(id: ResourceId) -> Self {
        Self { id }
    }
}

impl Handler<AddMutexMessage> for PacketDispatcher {
    type Result = Addr<DistMutex<PacketDispatcher>>;

    fn handle(&mut self, msg: AddMutexMessage, ctx: &mut Self::Context) -> Self::Result {
        println!("Adding mutex {}", msg.id);
        let id = msg.id;
        let mut mutex = DistMutex::new(id, ctx.address());
        self.connected_servers.iter().for_each(|server_id| {
            mutex.add_connected_server(*server_id);
        });
        let addr = mutex.start();
        self.mutexes.insert(id, addr.clone());
        addr
    }
}
