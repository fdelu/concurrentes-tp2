use std::collections::HashMap;
use crate::dist_mutex::{DistMutex, ResourceId, TCPActorTrait};

pub(crate) struct PacketDispatcher<T: TCPActorTrait> {
    mutexes: HashMap<ResourceId, DistMutex<T>>,

}