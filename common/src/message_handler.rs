use actix::{Actor, Context, Handler, Message};

/// Trait para un [Actor] que puede recibir un [Message] específico.
/// Similar a [Recipient](actix::Recipient), pero permite agregar
/// trait bounds de varios mensajes distintos.
pub trait AHandler<M: Message>: Handler<M> + Actor<Context = Context<Self>> {}
impl<A, M: Message> AHandler<M> for A where A: Handler<M> + Actor<Context = Context<Self>> {}
