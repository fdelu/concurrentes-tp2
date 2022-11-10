use crate::dist_mutex::packets::LockPacketType;
use crate::dist_mutex::{ResourceId, ServerId};

#[derive(Debug, Copy, Clone)]
pub(crate) struct RequestPacket {
    id: ResourceId,
    requester: ServerId,
}

impl RequestPacket {
    pub fn new(id: ResourceId, requester: ServerId) -> Self {
        Self {
            id,
            requester,
        }
    }
}

impl From<RequestPacket> for Vec<u8> {
    fn from(packet: RequestPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(LockPacketType::Request.into());
        let id: [u8; 4] = packet.id.into();
        buffer.extend(id.iter());
        let requester: [u8; 2] = packet.requester.into();
        buffer.extend(requester.iter());
        buffer
    }
}

impl TryFrom<Vec<u8>> for RequestPacket {
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
        let requester = value[9..11].try_into().unwrap();
        
        Ok(Self {
            id,
            requester,
        })
    }
}