use crate::dist_mutex::{ResourceId, ServerId};
use crate::dist_mutex::packets::LockPacketType;

pub struct OkPacket {
    id: ResourceId,
    server_id: ServerId,
}

impl OkPacket {
    pub fn new(id: ResourceId, server_id: ServerId) -> Self {
        Self { id, server_id }
    }
}

impl TryFrom<Vec<u8>> for OkPacket {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 17 {
            return Err(format!(
                "Invalid packet length: expected 17, got {}",
                value.len()
            ));
        }

        let packet_type = LockPacketType::try_from(value[0])?;
        if packet_type != LockPacketType::Ok {
            return Err(format!(
                "Invalid packet type: expected {:?}, got {:?}",
                LockPacketType::Ok, packet_type
            ));
        }

        let id = value[1..9].try_into().unwrap();
        let server_id = value[9..17].try_into().unwrap();

        Ok(Self { id, server_id })
    }
}

impl From<OkPacket> for Vec<u8> {
    fn from(packet: OkPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(LockPacketType::Ok.into());
        let id: [u8; 4] = packet.id.into();
        buffer.extend(id.iter());
        let server_id: [u8; 2] = packet.server_id.into();
        buffer.extend(server_id.iter());
        buffer
    }
}