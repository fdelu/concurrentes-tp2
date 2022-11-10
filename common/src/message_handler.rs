use actix::{Actor, Context, Handler, Message};
pub trait AHandler<M: Message>: Handler<M> + Actor<Context = Context<Self>> {}
impl<A, M: Message> AHandler<M> for A where A: Handler<M> + Actor<Context = Context<Self>> {}
