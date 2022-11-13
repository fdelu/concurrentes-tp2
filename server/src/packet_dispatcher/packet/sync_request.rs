use crate::dist_mutex::messages::Timestamp;

#[derive(Clone)]
pub struct SyncRequestPacket {
    pub timestamp: Timestamp,
}

impl From<SyncRequestPacket> for Vec<u8> {
    fn from(packet: SyncRequestPacket) -> Self {
        packet.timestamp.into()
    }
}

impl From<Vec<u8>> for SyncRequestPacket {
    fn from(data: Vec<u8>) -> Self {
        Self {
            timestamp: data[..].into(),
        }
    }
}