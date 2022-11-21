use actix::prelude::*;
use std::collections::HashSet;
use std::future::Future;
use crate::dist_mutex::packets::{Timestamp};
use crate::ServerId;
use crate::dist_mutex::MutexResult;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReleaseMessage;

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AckMessage {
    pub from: ServerId,
}

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct OkMessage {
    pub from: ServerId,
    pub connected_servers: HashSet<ServerId>,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub(crate) struct RequestMessage {
    pub from: ServerId,
    pub timestamp: Timestamp,
}

/// Mensaje para solicitar el lock del mutex distribuido.
/// Se debe utilizar con el método `send` de `Addr<DistMutex>`,
/// y esperar por el resultado con `await`.
/// El resultado es un future que se resuelve cuando el mutex
/// es adquirido.
/// Si el mutex no es adquirido en el tiempo especificado,
/// el future se resuelve con un error.
#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct AcquireMessage;


/// Ejecuta una función en exclusión mutua.
/// El mensaje adquiere el mutex y luego ejecuta la función.
/// Cuando la función termina, el mutex es liberado.
/// Si el mutex no puede ser adquirido, se devuelve un error.
#[derive(Message)]
#[rtype(result = "MutexResult<R>")]
pub struct DoWithLock<F, R, Fut>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R>,
    R: Send + 'static,
{
    pub action: F,
}
