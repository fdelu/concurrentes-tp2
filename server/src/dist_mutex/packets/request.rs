use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::MutexPacketType;
use crate::dist_mutex::{ResourceId, ServerId};

#[derive(Debug, Copy, Clone)]
pub struct RequestPacket {
    id: ResourceId,
    requester: ServerId,
    timestamp: Timestamp,
}

impl RequestPacket {
    pub fn new(id: ResourceId, requester: ServerId, timestamp: Timestamp) -> Self {
        Self {
            id,
            requester,
            timestamp,
        }
    }

    pub fn id(&self) -> ResourceId {
        self.id
    }

    pub fn requester(&self) -> ServerId {
        self.requester
    }

    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

impl From<RequestPacket> for Vec<u8> {
    fn from(packet: RequestPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Request.into());
        let id: [u8; 4] = packet.id.into();
        buffer.extend(id.iter());
        let requester: [u8; 2] = packet.requester.into();
        buffer.extend(requester.iter());
        let timestamp: [u8; 16] = packet.timestamp.into();
        buffer.extend(timestamp.iter());
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

        let packet_type = MutexPacketType::try_from(value[0])?;
        if packet_type != MutexPacketType::Request {
            return Err(format!(
                "Invalid packet type: expected {:?}, got {:?}",
                MutexPacketType::Request,
                packet_type
            ));
        }

        let id = value[1..9].try_into().unwrap();
        let requester = value[9..11].try_into().unwrap();
        let timestamp: Timestamp = value[11..27].into();

        Ok(Self {
            id,
            requester,
            timestamp,
        })
    }
}
