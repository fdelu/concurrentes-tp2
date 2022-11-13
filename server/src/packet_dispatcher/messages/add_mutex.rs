use actix::prelude::*;

use crate::{DistMutex, MutexCreationTrait, PacketDispatcher, ResourceId};

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
        let id = msg.id;
        let mutex = DistMutex::new(self.server_id, id, ctx.address());

        let addr = mutex.start();
        self.mutexes.insert(id, addr.clone());
        addr
    }
}
