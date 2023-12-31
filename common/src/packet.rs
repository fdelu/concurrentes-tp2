use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::error::CoffeeError;

/// ID de un usuario
pub type UserId = u32;
/// ID de una transacción para un usuario
pub type TxId = u32;
/// Cantidad de puntos
pub type Amount = u32;
/// ID de una cafetera
pub type CoffeeMakerId = SocketAddr;

/// Paquetes que envía la cafetera al servidor.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ClientPacket {
    RedeemCoffee(UserId, Amount, TxId),
    CommitRedemption(TxId),
    AddPoints(UserId, Amount, TxId),
}

/// Paquetes que envía el servidor a la cafetera.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ServerPacket {
    Ready(TxId),
    Insufficient(TxId),
    ServerError(TxId, CoffeeError),
}
