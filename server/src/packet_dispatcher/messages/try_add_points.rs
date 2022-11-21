use crate::packet_dispatcher::messages::add_points::AddPointsMessage;
use crate::packet_dispatcher::messages::public::queue_points::QueuePointsMessage;
use crate::packet_dispatcher::ADD_POINTS_ATTEMPT_INTERVAL;
use crate::PacketDispatcher;
use actix::prelude::*;
use tracing::{debug, trace};

#[derive(Message)]
#[rtype(result = "()")]
pub struct TryAddPointsMessage;

impl Handler<TryAddPointsMessage> for PacketDispatcher {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: TryAddPointsMessage, ctx: &mut Self::Context) -> Self::Result {
        trace!("Trying to add points [{:?}]", self.points_queue);

        let addr = ctx.address();
        let points_list = self.points_queue.clone();
        self.points_queue = vec![];

        async move {
            let mut not_added_points = vec![];
            for points in &points_list {
                if let Ok(Ok(())) = addr
                    .send(AddPointsMessage {
                        id: points.id,
                        amount: points.amount,
                    })
                    .await
                {
                    debug!("Added points to user {}", points.id);
                } else {
                    let msg = QueuePointsMessage {
                        id: points.id,
                        amount: points.amount,
                    };
                    not_added_points.push(msg);
                }
            }
            not_added_points
        }
        .into_actor(self)
        .then(move |not_added_points, me, ctx| {
            me.points_queue.extend(not_added_points);
            ctx.notify_later(TryAddPointsMessage, ADD_POINTS_ATTEMPT_INTERVAL);
            async {}.into_actor(me)
        })
        .boxed_local()
    }
}
