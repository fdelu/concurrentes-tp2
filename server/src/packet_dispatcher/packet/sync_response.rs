
pub struct SyncResponsePacket {
    // TODO: use clients info
    pub data: Vec<u8>,
}

impl From<SyncResponsePacket> for Vec<u8> {
    fn from(packet: SyncResponsePacket) -> Self {
        packet.data
    }
}

impl From<Vec<u8>> for SyncResponsePacket {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}