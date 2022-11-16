use actix::Actor;
use core::fmt::Display;
use crate::dist_mutex::{DistMutex};

impl<P: Actor> Display for DistMutex<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Mutex {}]", self.id)
    }
}


