use crate::dist_mutex::DistMutex;
use actix::Actor;
use core::fmt::Display;

impl<P: Actor> Display for DistMutex<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Mutex {}]", self.id)
    }
}
