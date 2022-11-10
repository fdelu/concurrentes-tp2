use crate::dist_mutex::packets::MutexPacketType;
use crate::dist_mutex::{ResourceId, ServerId};

pub struct OkPacket {
    id: ResourceId,
    sender: ServerId,
}

impl OkPacket {
    pub fn new(id: ResourceId, sender: ServerId) -> Self {
        Self { id, sender }
    }

    pub fn id(&self) -> ResourceId {
        self.id
    }

    pub fn sender(&self) -> ServerId {
        self.sender
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

        let packet_type = MutexPacketType::try_from(value[0])?;
        if packet_type != MutexPacketType::Ok {
            return Err(format!(
                "Invalid packet type: expected {:?}, got {:?}",
                MutexPacketType::Ok,
                packet_type
            ));
        }

        let id = value[1..9].try_into().unwrap();
        let sender = value[9..17].try_into().unwrap();

        Ok(Self { id, sender })
    }
}

impl From<OkPacket> for Vec<u8> {
    fn from(packet: OkPacket) -> Self {
        let mut buffer = Vec::new();
        buffer.push(MutexPacketType::Ok.into());
        let id: [u8; 4] = packet.id.into();
        buffer.extend(id.iter());
        let sender: [u8; 2] = packet.sender.into();
        buffer.extend(sender.iter());
        buffer
    }
}
