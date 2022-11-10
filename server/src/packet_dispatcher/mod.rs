use crate::dist_mutex::{DistMutex, ResourceId, TCPActorTrait};
use std::collections::HashMap;

pub(crate) struct PacketDispatcher<T: TCPActorTrait> {
    mutexes: HashMap<ResourceId, DistMutex<T>>,
}
