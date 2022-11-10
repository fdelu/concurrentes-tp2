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
        if value.len() != 9 {
            return Err(format!(
                "Invalid packet length: expected 9, got {}",
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

        let resource_id = value[1..9].try_into().unwrap();

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
