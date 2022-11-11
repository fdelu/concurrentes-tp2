use crate::dist_mutex::packets::MutexPacketType;
use crate::dist_mutex::ResourceId;

pub struct ReleasePacket {
    id: ResourceId,
}

impl ReleasePacket {
    pub fn new(resource_id: ResourceId) -> Self {
        Self { id: resource_id }
    }

    pub fn id(&self) -> ResourceId {
        self.id
    }
}

impl TryFrom<Vec<u8>> for ReleasePacket {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 5 {
            return Err(format!(
                "Invalid packet length: expected 5, got {}",
                value.len()
            ));
        }

        let packet_type = MutexPacketType::try_from(value[0])?;
        if packet_type != MutexPacketType::Release {
            return Err(format!(
                "Invalid packet type: expected {:?}, got {:?}",
                MutexPacketType::Release,
                packet_type
            ));
        }

        let resource_id = value[1..5].try_into().unwrap();

        Ok(Self { id: resource_id })
    }
}

impl From<ReleasePacket> for Vec<u8> {
    fn from(packet: ReleasePacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Release.into());
        let resource_id: [u8; 4] = packet.id.into();
        buffer.extend(resource_id.iter());
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_packet() {
        let resource_id = ResourceId::new(1);
        let packet = ReleasePacket::new(resource_id);
        let buffer: Vec<u8> = packet.into();
        let packet = ReleasePacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), resource_id);
    }

    #[test]
    fn test_serialize_packet() {
        let resource_id = ResourceId::new(1);
        let packet = ReleasePacket::new(resource_id);
        let buffer: Vec<u8> = packet.into();
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer[0], MutexPacketType::Release.into());
        let resource_id: [u8; 4] = resource_id.into();
        assert_eq!(buffer[1..5], resource_id);
    }

    #[test]
    fn test_deserialize_packet() {
        let resource_id = ResourceId::new(1);
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Release.into());
        let resource_id: [u8; 4] = resource_id.into();
        buffer.extend(resource_id.iter());
        let packet = ReleasePacket::try_from(buffer).unwrap();
        let packet_id: [u8; 4] = packet.id().into();
        assert_eq!(packet_id, resource_id);
    }

    #[test]
    fn test_deserialize_invalid_packet() {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Ok.into());
        let resource_id = ResourceId::new(1);
        let resource_id: [u8; 4] = resource_id.into();
        buffer.extend(resource_id.iter());
        let result = ReleasePacket::try_from(buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_invalid_length() {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Release.into());
        let resource_id = ResourceId::new(1);
        let resource_id: [u8; 4] = resource_id.into();
        buffer.extend(resource_id.iter());
        buffer.push(0);
        let result = ReleasePacket::try_from(buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_invalid_type() {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Ok.into());
        let resource_id = ResourceId::new(1);
        let resource_id: [u8; 4] = resource_id.into();
        buffer.extend(resource_id.iter());
        let result = ReleasePacket::try_from(buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_deserialize() {
        let resource_id = ResourceId::new(1);
        let packet = ReleasePacket::new(resource_id);
        let buffer: Vec<u8> = packet.into();
        let packet = ReleasePacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), resource_id);
    }
}
