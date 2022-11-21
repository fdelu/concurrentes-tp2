use crate::dist_mutex::packets::Timestamp;
use crate::packet_dispatcher::TransactionId;
use crate::two_phase_commit::packets::Transaction;
use crate::two_phase_commit::{CommitResult, TransactionState, UserData};
use crate::ServerId;
use actix::prelude::*;
use common::packet::UserId;
use std::collections::{HashMap, HashSet};

/// Mensaje para ejecutar la primera fase del
/// algoritmo de dos fases de commit.
/// El actor envía un `PreparePacket` a todos los
/// servidores conectados, que se convierte en el mensaje
/// `PrepareMessage` para cada uno de ellos.
/// Devuelve `Ok(true)` si todos los servidores responden
/// con un `VoteYesPacket`, `Ok(false)` si al menos uno
/// responde con un `VoteNoPacket`, y `Err` si no se
/// reciben respuestas de todos los servidores, o si
/// ocurre algún error al enviar el mensaje.
#[derive(Message)]
#[rtype(result = "CommitResult<bool>")]
pub struct CommitRequestMessage {
    /// Id de la transacción.
    /// Se usa para identificar la transacción en los
    /// mensajes de los servidores.
    pub id: TransactionId,
    /// Información de la transacción que se quiere
    /// ejecutar.
    pub transaction: Transaction,
}

/// Mensaje para ejecutar la segunda fase del
/// algoritmo de dos fases de commit.
/// El actor envía un `CommitPacket` a todos los
/// servidores conectados, que se convierte en el mensaje
/// `CommitMessage` para cada uno de ellos.
/// Devuelve `Ok(())` si se ejecuta correctamente, y `Err`
/// en caso de que algún servidor haya respondido con un
/// `VoteNoPacket`.
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct CommitCompleteMessage {
    pub id: TransactionId,
    /// Lista de los servidores que se consideran
    /// conectados al sistema, pues han respondido
    /// a mensajes previos. Esto es necesario para
    /// determinar si todos los servidores han
    /// respondido o no.
    pub connected_servers: HashSet<ServerId>,
}

/// Mensaje que recibe un actor cuando un servidor
/// responde con un `VoteYesPacket` a un `PreparePacket`.
/// Si todos los servidores responden con un `VoteYesPacket`,
/// el actor puede ejecutar la segunda fase del algoritmo
#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct VoteYesMessage {
    /// Id del servidor que ha respondido.
    pub from: ServerId,
    /// Id de la transacción.
    pub id: TransactionId,
    /// Lista de los servidores que se consideran
    /// conectados al sistema.
    pub connected_servers: HashSet<ServerId>,
}

/// Mensaje que recibe un actor cuando un servidor
/// responde con un `VoteNoPacket` a un `PreparePacket`.
/// Si al menos un servidor responde con un `VoteNoPacket`,
/// el actor no puede ejecutar la segunda fase del algoritmo.
/// No se envía un mensaje de Rollback a los servidores,
/// pues el rollback se ejecuta automáticamente al no
/// recibir un `CommitPacket` luego de un tiempo.
#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct VoteNoMessage {
    /// Id del servidor que ha respondido.
    pub from: ServerId,
    /// Id de la transacción.
    pub id: TransactionId,
}

/// Mensaje que recibe un actor de un servidor
/// como respuesta de haber solicitado una actualización
/// de la base de datos con `SyncRequestMessage`.
#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateDatabaseMessage {
    /// Momento en que se realizó la última actualización
    /// de la base de datos.
    pub snapshot_from: Timestamp,
    /// Base de datos actualizada.
    pub database: HashMap<UserId, UserData>,
    /// Lista de logs con las transacciones que se han
    /// ejecutado o abortado hasta el momento, y que aún están
    /// pendientes de ser confirmadas.
    /// En caso de que alguna transacción tenga un estado
    /// `TransactionState::Prepared`, se crea un timer para
    /// ejecutar el rollback de la transacción, en caso de que
    /// no se reciba un `CommitPacket` en un tiempo determinado.
    pub logs: HashMap<TransactionId, (TransactionState, Transaction)>,
}

/// Mensaje que recibe un actor para deshacer
/// una transacción que no se ha completado.
/// El estado de la transacción pasa a ser `TransactionState::Aborted`.
#[derive(Message)]
#[rtype(result = "()")]
pub struct RollbackMessage {
    /// Id de la transacción que se quiere deshacer.
    pub id: TransactionId,
}

/// Mensaje que un actor se envía a sí mismo
/// para abortar una transacción que no se ha completado.
#[derive(Message)]
#[rtype(result = "()")]
pub struct TransactionTimeoutMessage {
    /// Id de la transacción que se quiere deshacer.
    pub transaction_id: TransactionId,
}

/// Mensaje que recibe un actor cuando un servidor
/// quiere realizar una actualización de la base de datos.
/// El actor responde con un `VoteYesPacket` si acepta la
/// actualización, o con un `VoteNoPacket` si no.
#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct PrepareMessage {
    /// Id del servidor que ha enviado el mensaje.
    pub from: ServerId,
    /// Id de la transacción.
    pub id: TransactionId,
    /// Información de la transacción que se quiere
    /// ejecutar.
    pub transaction: Transaction,
}

/// Mensaje que recibe un actor cuando un servidor
/// recibe un `SyncRequestPacket`.
/// El actor responde con un `SyncResponsePacket` con la
/// base de datos actualizada.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ForwardDatabaseMessage {
    /// Id del servidor que ha enviado el mensaje.
    /// Se usa para responder con un `SyncResponsePacket`.
    pub to: ServerId,
}

/// Mensaje que recibe un actor cuando un servidor
/// recibe un `CommitPacket`.
/// Significa que la transacción se ha completado
/// y se puede confirmar.
#[derive(Message)]
#[rtype(result = "CommitResult<()>")]
pub struct CommitMessage {
    /// Id del servidor que ha enviado el mensaje.
    pub from: ServerId,
    /// Id de la transacción.
    pub id: TransactionId,
}
