use super::error::PacketDispatcherResult;
use crate::dist_mutex::packets::Timestamp;
use crate::packet_dispatcher::packet::Packet;
use crate::packet_dispatcher::TransactionId;
use crate::ServerId;
use actix::prelude::*;
use common::packet::UserId;
use common::socket::SocketError;

/// Mensaje que recibe un actor cuando se quiere
/// bloquear cierta cantidad de puntos de un usuario
/// Esto inicia la primera fase del algoritmo de commit
/// Devuelve [Ok(())] si se pudieron bloquear los puntos.
/// Si el usuario no tiene suficientes puntos, devuelve
/// [Err(PacketDispatcherError::InsufficientPoints)]
#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct BlockPointsMessage {
    /// Id de la transacción que se quiere realizar
    pub transaction_id: TransactionId,
    /// Id del usuario al que se le quieren bloquear los puntos
    pub user_id: UserId,
    /// Cantidad de puntos que se quieren bloquear
    pub amount: u32,
}

/// Reinicia el estado interno del actor,
/// sin afectar a los puntos que aún no
/// fueron agregados al usuario.
#[derive(Message)]
#[rtype(result = "()")]
pub struct DieMessage;

/// Ejecuta la segunda fase del algoritmo de commit,
/// confirmando el descuento de puntos
#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct DiscountMessage {
    /// Id del usuario al que se le quieren descontar los puntos
    pub user_id: UserId,
    /// Id de la transacción que se quiere realizar
    pub transaction_id: TransactionId,
}

/// Encola una cantidad de puntos a ser agregados
/// al cliente.
/// Si el sistema se encuentra desconectado,
/// se intentarán agregar los puntos cada cierto tiempo
/// hasta que se pueda hacer.
#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct QueuePointsMessage {
    /// Id del usuario al que se le quieren agregar los puntos
    pub id: UserId,
    /// Cantidad de puntos que se quieren agregar
    pub amount: u32,
}

impl QueuePointsMessage {
    /// Convierte el mensaje [QueuePointsMessage] en un mensaje
    /// [AddPointsMessage] que puede ser enviado al
    /// [PacketDispatcher] para intentar agregar los puntos
    pub fn to_add_points_msg(&self) -> AddPointsMessage {
        AddPointsMessage {
            id: self.id,
            amount: self.amount,
        }
    }
}

/// Envía un paquete a todos los servidores
/// conectados al sistema
#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct BroadcastMessage {
    /// Paquete que se quiere enviar
    pub packet: Packet,
}

/// Modifica el estado de los servidores,
/// desconectando a aquellos cuya última
/// actividad ocurrió antes de [older_than]
#[derive(Message)]
#[rtype(result = "()")]
pub struct PruneMessage {
    /// Tiempo por el cual se quiere filtrar
    /// la lista de servidores
    pub older_than: Timestamp,
}

/// Envía un paquete a un servidor en particular
#[derive(Message)]
#[rtype(result = "Result<(), SocketError>")]
pub struct SendMessage {
    /// Id del servidor al que se quiere enviar el paquete
    pub to: ServerId,
    /// Paquete que se quiere enviar
    pub packet: Packet,
}

/// Intenta agregar los puntos que se encuentran
/// encolados.
/// Luego de intentarlo, se envía recursivamente
/// al mismo actor para ejecutarlo luego de un tiempo
#[derive(Message)]
#[rtype(result = "()")]
pub struct TryAddPointsMessage;

/// Agrega una cantidad de puntos a un usuario
/// Para ello, adquiere el lock correspondiente
/// al usuario y luego ejecuta el algoritmo de commit
/// Devuelve [Ok(())] si se pudieron agregar los puntos.
/// De lo contrario, devuelve un error
#[derive(Message)]
#[rtype(result = "PacketDispatcherResult<()>")]
pub struct AddPointsMessage {
    pub id: UserId,
    pub amount: u32,
}
