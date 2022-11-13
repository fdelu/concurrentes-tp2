#[derive(Debug)]
pub struct SyncResponsePacket {
    // TODO: use clients info
    pub data: Vec<u32>,
}

impl From<SyncResponsePacket> for Vec<u8> {
    fn from(packet: SyncResponsePacket) -> Self {
        let mut data = Vec::new();
        for value in packet.data {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data
    }
}

impl From<Vec<u8>> for SyncResponsePacket {
    fn from(data: Vec<u8>) -> Self {
        let mut packet = Self { data: Vec::new() };
        for i in (0..data.len()).step_by(4) {
            packet.data.push(u32::from_be_bytes([
                data[i],
                data[i + 1],
                data[i + 2],
                data[i + 3],
            ]));
        }
        packet
    }
}
