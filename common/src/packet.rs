use serde::{Deserialize, Serialize};

use crate::error::CoffeeError;

/// ID de un usuario
pub type UserId = u32;
/// ID de una transacción para un usuario
pub type TxId = u32;
/// Cantidad de puntos
pub type Amount = u32;

/// Paquetes que envía la cafetera al servidor.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ClientPacket {
    PrepareOrder(UserId, Amount, TxId),
    CommitOrder(TxId),
    AddPoints(UserId, Amount, TxId),
}

/// Paquetes que envía el servidor a la cafetera.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ServerPacket {
    Ready(TxId),
    Insufficient(TxId),
    ServerErrror(TxId, CoffeeError),
}
