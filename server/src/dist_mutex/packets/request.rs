use crate::dist_mutex::packets::LockPacketType;
use crate::dist_mutex::ResourceId;

#[derive(Debug, Copy, Clone)]
pub(crate) struct LockRequestPacket {
    id: ResourceId,
}

impl LockRequestPacket {
    pub fn new(id: ResourceId) -> Self {
        Self { id }
    }
}

impl From<LockRequestPacket> for Vec<u8> {
    fn from(packet: LockRequestPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(LockPacketType::Request.into());
        let id: [u8; 4] = packet.id.into();
        buffer.extend(id.iter());
        buffer
    }
}

impl TryFrom<Vec<u8>> for LockRequestPacket {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 9 {
            return Err(format!(
                "Invalid packet length: expected 9, got {}",
                value.len()
            ));
        }

        let packet_type = LockPacketType::try_from(value[0])?;
        if packet_type != LockPacketType::Request {
            return Err(format!(
                "Invalid packet type: expected {:?}, got {:?}",
                LockPacketType::Request, packet_type
            ));
        }

        let id = value[1..9].try_into().unwrap();

        Ok(Self { id })
    }
}