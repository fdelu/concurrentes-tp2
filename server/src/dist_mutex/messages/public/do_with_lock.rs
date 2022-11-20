use std::future::Future;
use actix::prelude::*;
use common::AHandler;
use crate::dist_mutex::{DistMutex, MutexError};
use crate::dist_mutex::MutexResult;
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::messages::prune::PruneMessage;
use crate::packet_dispatcher::messages::send::SendMessage;

#[derive(Message)]
#[rtype(result = "MutexResult<R>")]
pub struct DoWithLock<F, R, Fut>
where F: FnOnce() -> Fut + Send + 'static,
      Fut: Future<Output = R>,
      R: Send + 'static
{
    pub action: F
}

impl<F, P, R, Fut> Handler<DoWithLock<F, R, Fut>> for DistMutex<P>
where
    F: FnOnce() -> Fut + Send + 'static,
    P: AHandler<BroadcastMessage> + AHandler<SendMessage> + AHandler<PruneMessage>,
    Fut: Future<Output = R>,
    R: Send + 'static
{
    type Result = ResponseActFuture<Self, MutexResult<R>>;

    fn handle(&mut self, msg: DoWithLock<F, R, Fut>, ctx: &mut Self::Context) -> Self::Result {
        let addr = ctx.address();

        async move {
            if addr.send(crate::AcquireMessage {}).await.is_err() {
                return Err(MutexError::Timeout);
            };
            let r = (msg.action)().await;
            if addr.send(crate::ReleaseMessage {}).await.is_err() {
                return Err(MutexError::Timeout);
            };
            Ok(r)
        }.into_actor(self).boxed_local()
    }
}