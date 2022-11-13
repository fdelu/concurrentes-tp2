mod sync_request;
mod sync_response;
pub use sync_request::SyncRequestPacket;
pub use sync_response::SyncResponsePacket;

const MUTEX_TYPE: u8 = 0;
const COMMIT_TYPE: u8 = 1;
const SYNC_REQUEST_TYPE: u8 = 2;
const SYNC_RESPONSE_TYPE: u8 = 3;

#[derive(Clone, Copy, Debug)]
pub enum PacketType {
    Mutex,
    Commit,
    SyncRequest,
    SyncResponse,
}

impl TryFrom<u8> for PacketType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            MUTEX_TYPE => Ok(PacketType::Mutex),
            COMMIT_TYPE => Ok(PacketType::Commit),
            SYNC_REQUEST_TYPE => Ok(PacketType::SyncRequest),
            SYNC_RESPONSE_TYPE => Ok(PacketType::SyncResponse),
            _ => Err(format!("Invalid packet type: {}", value)),
        }
    }
}

impl From<PacketType> for u8 {
    fn from(value: PacketType) -> Self {
        match value {
            PacketType::Mutex => MUTEX_TYPE,
            PacketType::Commit => COMMIT_TYPE,
            PacketType::SyncRequest => SYNC_REQUEST_TYPE,
            PacketType::SyncResponse => SYNC_RESPONSE_TYPE,
        }
    }
}
