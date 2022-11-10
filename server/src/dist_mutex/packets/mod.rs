use crate::dist_mutex::{ResourceId};

mod request;

pub(crate) use request::LockRequestPacket;

const LOCK_REQUEST_TYPE: u8 = 0;
const LOCK_ACK_TYPE: u8 = 1;
const LOCK_OK_TYPE: u8 = 2;
const LOCK_RELEASE_TYPE: u8 = 3;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LockPacketType {
    Request,
    Lock,
    Ok,
    Release,
}

impl From<LockPacketType> for u8 {
    fn from(packet_type: LockPacketType) -> Self {
        match packet_type {
            LockPacketType::Request => LOCK_REQUEST_TYPE,
            LockPacketType::Lock => LOCK_ACK_TYPE,
            LockPacketType::Ok => LOCK_OK_TYPE,
            LockPacketType::Release => LOCK_RELEASE_TYPE,
        }
    }
}

impl TryFrom<u8> for LockPacketType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            LOCK_REQUEST_TYPE => Ok(LockPacketType::Request),
            LOCK_ACK_TYPE => Ok(LockPacketType::Lock),
            LOCK_OK_TYPE => Ok(LockPacketType::Ok),
            LOCK_RELEASE_TYPE => Ok(LockPacketType::Release),
            _ => Err(format!("Unknown packet type: {}", value)),
        }
    }
}

