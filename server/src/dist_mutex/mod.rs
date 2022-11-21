use crate::dist_mutex::packets::{MutexPacket, ResourceId, Timestamp};
use crate::dist_mutex::server_id::ServerId;
use crate::packet_dispatcher::messages::prune::PruneMessage;
use crate::packet_dispatcher::messages::public::die::DieMessage;
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::Packet;
use actix::prelude::*;
use common::error::FlattenResult;
use common::AHandler;
use std::collections::HashSet;
use std::fmt::Display;
use std::time::Duration;
use tokio::sync::oneshot;
use actix::fut::LocalBoxActorFuture;
use tokio::time;
use tracing::debug;

pub mod messages;
pub mod packets;
pub mod server_id;
pub mod messages_impls;

#[cfg(not(test))]
const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_secs(10);
#[cfg(test)]
const TIME_UNTIL_DISCONNECT_POLITIC: Duration = Duration::from_millis(10);

#[cfg(not(test))]
const TIME_UNTIL_ERROR: Duration = Duration::from_secs(60);
#[cfg(test)]
const TIME_UNTIL_ERROR: Duration = Duration::from_millis(10);

/// Actor que implementa el algoritmo distribuido
/// para la obtención de un mutex.
/// Cada cuenta de usuario representa una sección crítica
/// que debe ser accedida mediante un mutex diferente.
pub struct DistMutex<P: Actor> {
    /// Id del servidor asociado al mutex
    /// Es usado para armar los paquetes que se enviarán
    /// a los otros servidores.
    server_id: ServerId,
    /// Id del recurso que protege el mutex,
    /// en este caso, el id del usuario.
    id: ResourceId,
    /// Dispatcher asociado al mutex.
    /// Es usado para enviar los paquetes a los otros servidores
    /// (Broadcast para el `RequestMessage`, Send para el resto).
    dispatcher: Addr<P>,
    /// Cola con todos los servidores que han solicitado el mutex.
    /// Un servidor se encola cuando el DistMutex recibe un `RequestMessage`
    /// y ocurre alguno de los siguientes casos:
    /// - El mutex está adquirido por dicho actor
    /// - El actor está esperando a que el mutex sea liberado, y su timestamp
    /// es menor al del mensaje recibido
    /// Cuando el mutex es liberado, se envía un `OkMessage` a todos los servidores
    /// en la cola, y se vacía la cola.
    queue: Vec<(Timestamp, ServerId)>,
    /// Timestamp del momento en que se comenzó a esperar
    /// por el mutex.
    /// Si el actor no está esperando, el valor es `None`.
    lock_timestamp: Option<Timestamp>,
    /// Servidores que han enviado un `AckMessage` al mutex.
    ack_received: HashSet<ServerId>,
    /// Servidores que han enviado un `OkMessage` al mutex.
    /// Si coincide con `ack_received`, significa que el actor
    /// tiene permiso para adquirir el mutex.
    ok_received: HashSet<ServerId>,
    /// Canal utilizado para esperar la recepción de los `AckMessage`
    /// y `OkMessage` necesarios para adquirir el mutex.
    all_oks_received_channel: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum MutexError {
    /// El mutex no pudo ser adquirido en el tiempo establecido.
    Timeout,
    /// El sistema se encuentra desconectado de la red.
    Disconnected,
    /// Ocurrió un error al enviar un mensaje a otro actor.
    Mailbox(String),
}

impl<T> FlattenResult<T, MutexError> for MutexError {
    fn flatten(self) -> Result<T, MutexError> {
        Err(self)
    }
}

impl From<MailboxError> for MutexError {
    fn from(e: MailboxError) -> Self {
        MutexError::Mailbox(e.to_string())
    }
}

type MutexResult<T> = Result<T, MutexError>;

pub trait MutexCreationTrait<P: Actor> {
    fn new(server_id: ServerId, id: ResourceId, dispatcher: Addr<P>) -> Self;
}

impl<P: Actor> MutexCreationTrait<P> for DistMutex<P> {
    fn new(server_id: ServerId, id: ResourceId, dispatcher: Addr<P>) -> Self {
        Self {
            server_id,
            id,
            dispatcher,
            lock_timestamp: None,
            ack_received: HashSet::new(),
            ok_received: HashSet::new(),
            all_oks_received_channel: None,
            queue: Vec::new(),
        }
    }
}

impl<P: Actor> Actor for DistMutex<P> {
    type Context = Context<Self>;
}

impl<P: Actor> DistMutex<P> {
    /// Limpia el estado del mutex para que pueda ser usado nuevamente.
    fn clean_state(&mut self) {
        self.ack_received.clear();
        self.ok_received.clear();
        self.all_oks_received_channel = None;
    }
}

/// Envía un paquete al servidor especificado.
/// Parámetros:
/// - `dispatcher_addr`: dirección del dispatcher asociado al mutex.
/// - `server_id`: Id del servidor al que se enviará el paquete.
/// - `packet`: Paquete a enviar.
fn do_send<P>(dispatcher_addr: &Addr<P>, to: ServerId, packet: MutexPacket)
where
    P: AHandler<SendMessage>
{
    let packet = Packet::Mutex(packet);
    dispatcher_addr.do_send(SendMessage { to, packet });
}

impl<P: AHandler<PruneMessage>> DistMutex<P> {
    /// Envía un mensaje `PruneMessage` al dispatcher,
    /// para que éste elimine los servidores que no respondieron
    /// a los mensajes `RequestMessage` enviados.
    fn send_prune(&mut self) {
        let message = PruneMessage {
            older_than: self.lock_timestamp.unwrap(),
        };
        self.dispatcher.do_send(message);
    }
}

impl<P: Actor> Handler<DieMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, _: DieMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

impl<P: Actor> Supervised for DistMutex<P> {
    fn restarting(&mut self, _: &mut Self::Context) {
        self.clean_state();
    }
}

impl<P: Actor> DistMutex<P> {
    fn box_fut<T>(&mut self, inner: T) -> LocalBoxActorFuture<Self, T>
    where
        T: 'static,
    {
        async move { inner }.into_actor(self).boxed_local()
    }
}

impl<P: Actor> DistMutex<P> {
    fn wait_for_oks(
        &mut self,
        dur: Duration,
    ) -> LocalBoxActorFuture<DistMutex<P>, Result<(), MutexError>> {
        debug!("{} Waiting {} ms for lock", self, dur.as_millis());
        let (tx, rx) = oneshot::channel();
        self.all_oks_received_channel = Some(tx);
        async move {
            if time::timeout(dur, rx).await.is_err() {
                debug!("Timeout while waiting for oks");
                Err(MutexError::Timeout)
            } else {
                debug!("All oks received");
                Ok(())
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl<P: Actor> Display for DistMutex<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Mutex {}]", self.id)
    }
}
