use crate::dist_mutex::packets::MutexPacketType;
use crate::dist_mutex::{ResourceId, ServerId};

pub struct AckPacket {
    id: ResourceId,
}

impl AckPacket {
    pub fn new(id: ResourceId) -> Self {
        Self {
            id,
        }
    }

    pub fn id(&self) -> ResourceId {
        self.id
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

        let packet_type = MutexPacketType::try_from(value[0])?;
        if packet_type != MutexPacketType::Ack {
            return Err(format!(
                "Invalid packet type: expected {:?}, got {:?}",
                MutexPacketType::Ack,
                packet_type
            ));
        }

        let id = value[1..9].try_into().unwrap();

        Ok(Self {
            id,
        })
    }
}

impl From<AckPacket> for Vec<u8> {
    fn from(_: AckPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Ack.into());

        buffer
    }
}
