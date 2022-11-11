use crate::dist_mutex::packets::MutexPacketType;
use crate::dist_mutex::{ResourceId, ServerId};

pub struct AckPacket {
    id: ResourceId,
}

impl AckPacket {
    pub fn new(id: ResourceId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> ResourceId {
        self.id
    }
}

impl TryFrom<Vec<u8>> for AckPacket {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 5 {
            return Err(format!(
                "Invalid packet length: expected 5, got {}",
                value.len()
            ));
        }

        let packet_type = MutexPacketType::try_from(value[0])?;
        if packet_type != MutexPacketType::Ack {
            return Err(format!(
                "Invalid packet type: expected {:?}, got {:?}",
                MutexPacketType::Ack,
                packet_type
            ));
        }

        let id = value[1..5].try_into().unwrap();

        Ok(Self { id })
    }
}

impl From<AckPacket> for Vec<u8> {
    fn from(packet: AckPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Ack.into());
        let id: [u8; 4] = packet.id.into();
        buffer.extend(id.iter());
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_packet() {
        let id = ResourceId::new(1);
        let packet = AckPacket::new(id);
        assert_eq!(packet.id(), id);
    }

    #[test]
    fn test_serialize_packet() {
        let id = ResourceId::new(1);
        let packet = AckPacket::new(id);
        let buffer = Vec::from(packet);
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer[0], MutexPacketType::Ack.into());
        assert_eq!(buffer[1..5], [0, 0, 0, 1]);
    }

    #[test]
    fn test_deserialize_packet() {
        let buffer = vec![MutexPacketType::Ack.into(), 0, 0, 0, 1];
        let packet = AckPacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), ResourceId::new(1));
    }

    #[test]
    fn test_deserialize_invalid_packet() {
        let buffer = vec![MutexPacketType::Ok.into(), 0, 0, 0, 1];
        let packet = AckPacket::try_from(buffer);
        assert!(packet.is_err());
    }

    #[test]
    fn test_deserialize_invalid_length() {
        let buffer = vec![MutexPacketType::Ack.into(), 0, 0, 0];
        let packet = AckPacket::try_from(buffer);
        assert!(packet.is_err());
    }

    #[test]
    fn test_serialize_deserialize() {
        let id = ResourceId::new(1);
        let packet = AckPacket::new(id);
        let buffer = Vec::from(packet);
        let packet = AckPacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), id);
    }
}
