use serde::{Deserialize, Serialize};

use crate::socket::SocketError;

pub type UserId = u32;
pub type TxId = u32;
pub type Amount = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ClientPacket {
    PrepareOrder(UserId, Amount, TxId),
    CommitOrder(TxId),
    AddPoints(UserId, Amount, TxId),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ServerPacket {
    Ready(TxId),
    Insufficient(TxId),
    ServerErrror(TxId, SocketError),
}
