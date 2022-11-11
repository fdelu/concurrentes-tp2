use crate::dist_mutex::messages::Timestamp;
use crate::dist_mutex::packets::MutexPacketType;
use crate::dist_mutex::{ResourceId, ServerId};

#[derive(Debug, Copy, Clone)]
pub struct RequestPacket {
    id: ResourceId,
    timestamp: Timestamp,
}

impl RequestPacket {
    pub fn new(id: ResourceId, timestamp: Timestamp) -> Self {
        Self { id, timestamp }
    }

    pub fn id(&self) -> ResourceId {
        self.id
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
        let timestamp: [u8; 16] = packet.timestamp.into();
        buffer.extend(timestamp.iter());
        buffer
    }
}

impl TryFrom<Vec<u8>> for RequestPacket {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 21 {
            return Err(format!(
                "Invalid packet length: expected 21, got {}",
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

        let id = value[1..5].into();
        let timestamp: Timestamp = value[5..21].into();

        Ok(Self { id, timestamp })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_packet() {
        let id = ResourceId::new(1);
        let timestamp = Timestamp::new();
        let packet = RequestPacket::new(id, timestamp);
        let buffer: Vec<u8> = packet.into();
        let packet = RequestPacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), id);
        assert_eq!(packet.timestamp(), timestamp);
    }

    #[test]
    fn test_invalid_packet() {
        let id = ResourceId::new(1);
        let timestamp = Timestamp::new();
        let packet = RequestPacket::new(id, timestamp);
        let mut buffer: Vec<u8> = packet.into();
        buffer[0] = 1;
        let packet = RequestPacket::try_from(buffer);
        assert!(packet.is_err());
    }

    #[test]
    fn test_invalid_length() {
        let id = ResourceId::new(1);
        let timestamp = Timestamp::new();
        let packet = RequestPacket::new(id, timestamp);
        let mut buffer: Vec<u8> = packet.into();
        buffer.pop();
        let packet = RequestPacket::try_from(buffer);
        assert!(packet.is_err());
    }

    #[test]
    fn test_serialize_deserialize() {
        let id = ResourceId::new(1);
        let timestamp = Timestamp::new();
        let packet = RequestPacket::new(id, timestamp);
        let buffer: Vec<u8> = packet.into();
        let packet = RequestPacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), id);
        assert_eq!(packet.timestamp(), timestamp);
    }
}
