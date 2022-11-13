use crate::dist_mutex::packets::{decode_mutex_packet_with_only_id, MutexPacketType};
use crate::dist_mutex::ResourceId;

pub struct OkPacket {
    id: ResourceId,
}

impl OkPacket {
    pub fn new(id: ResourceId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> ResourceId {
        self.id
    }
}

impl TryFrom<Vec<u8>> for OkPacket {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self {
            id: decode_mutex_packet_with_only_id(value, MutexPacketType::Ok)?,
        })
    }
}

impl From<OkPacket> for Vec<u8> {
    fn from(packet: OkPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Ok.into());
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
        let packet = OkPacket::new(id);
        let buffer = Vec::from(packet);
        let packet = OkPacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), id);
    }

    #[test]
    fn test_serialize_packet() {
        let id = ResourceId::new(1);
        let packet = OkPacket::new(id);
        let buffer = Vec::from(packet);
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer[0], MutexPacketType::Ok.into());
        assert_eq!(buffer[1..5], [0, 0, 0, 1]);
    }

    #[test]
    fn test_deserialize_packet() {
        let buffer = vec![MutexPacketType::Ok.into(), 0, 0, 0, 1];
        let packet = OkPacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), ResourceId::new(1));
    }

    #[test]
    fn test_deserialize_invalid_packet() {
        let buffer = vec![MutexPacketType::Ack.into(), 0, 0, 0, 1];
        let packet = OkPacket::try_from(buffer);
        assert!(packet.is_err());
    }

    #[test]
    fn test_deserialize_invalid_length() {
        let buffer = vec![MutexPacketType::Ok.into(), 0, 0, 0];
        let packet = OkPacket::try_from(buffer);
        assert!(packet.is_err());
    }

    #[test]
    fn test_deserialize_invalid_type() {
        let buffer = vec![MutexPacketType::Ack.into(), 0, 0, 0, 1];
        let packet = OkPacket::try_from(buffer);
        assert!(packet.is_err());
    }

    #[test]
    fn test_deserialize_invalid_length_and_type() {
        let buffer = vec![MutexPacketType::Ack.into(), 0, 0, 0];
        let packet = OkPacket::try_from(buffer);
        assert!(packet.is_err());
    }

    #[test]
    fn test_serialize_deserialize() {
        let id = ResourceId::new(1);
        let packet = OkPacket::new(id);
        let buffer = Vec::from(packet);
        let packet = OkPacket::try_from(buffer).unwrap();
        assert_eq!(packet.id(), id);
    }
}
