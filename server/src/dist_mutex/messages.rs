use crate::dist_mutex::packets::Timestamp;
use crate::dist_mutex::MutexResult;
use crate::ServerId;
use actix::prelude::*;
use std::collections::HashSet;
use std::future::Future;

/// Mensaje para liberar un mutex
/// Cuando se recibe, el actor envía un mensaje de `OkMessage`
/// a todos los servidores que estén esperando el mutex, y vacía la cola.
/// Si el mutex no está adquirido, no hace nada.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ReleaseMessage;

/// Mensaje para indicar que un servidor ha recibido el `RequestPacket`
#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AckMessage {
    /// Id del servidor que envió el Acknowledge
    pub from: ServerId,
}

/// Mensaje para indicar que un servidor confirmó la solicitud
/// de adquisición del mutex.
/// Cuando el servidor que envió el `RequestMessage` recibe todos los
/// `OkMessage`s, significa que el mutex fue adquirido.
#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct OkMessage {
    /// Id del servidor que envió el Ok
    pub from: ServerId,
    /// Lista con todos los servidores que se consideran conectados.
    /// Es utilizada para determinar si todos los servidores han enviado
    /// un `OkMessage`.
    pub connected_servers: HashSet<ServerId>,
}

/// Mensaje que recibe un actor cuando un servidor
/// está intentando adquirir el mutex.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub(crate) struct RequestMessage {
    /// Id del servidor que quiere adquirir el mutex
    pub from: ServerId,
    /// Timestamp del momento en que el servidor empezó a esperar
    /// por el mutex.
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
