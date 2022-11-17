use serde::{Deserialize, Serialize};

pub type UserId = String;
pub type Amount = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ClientPacket {
    PrepareOrder(UserId, Amount),
    CommitOrder(),
    AbortOrder(),
    AddPoints(UserId, Amount),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ServerPacket {
    Ready(),
    Insufficient(),
}
