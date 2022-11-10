use crate::dist_mutex::packets::LockPacketType;
use crate::dist_mutex::{ResourceId, ServerId};

pub struct AckPacket {
    id: ResourceId,
    server_id: ServerId,
}

impl AckPacket {
    pub fn new(id: ResourceId, server_id: ServerId) -> Self {
        Self { id, server_id }
    }
}

impl TryFrom<Vec<u8>> for AckPacket {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 9 {
            return Err(format!(
                "Invalid packet length: expected 9, got {}",
                value.len()
            ));
        }

        let packet_type = LockPacketType::try_from(value[0])?;
        if packet_type != LockPacketType::Ack {
            return Err(format!(
                "Invalid packet type: expected {:?}, got {:?}",
                LockPacketType::Ack,
                packet_type
            ));
        }

        let id = value[1..9].try_into().unwrap();
        let server_id = value[9..17].try_into().unwrap();

        Ok(Self { id, server_id })
    }
}

impl From<AckPacket> for Vec<u8> {
    fn from(packet: AckPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(LockPacketType::Ack.into());
        let server_id: [u8; 2] = packet.server_id.into();
        buffer.extend(server_id.iter());
        buffer
    }
}
