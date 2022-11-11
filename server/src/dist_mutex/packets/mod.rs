mod ack;
mod ok;
mod release;
mod request;

pub(crate) use ack::AckPacket;
pub(crate) use ok::OkPacket;
pub(crate) use release::ReleasePacket;
pub(crate) use request::RequestPacket;

const MUTEX_REQUEST_TYPE: u8 = 0;
const MUTEX_ACK_TYPE: u8 = 1;
const MUTEX_OK_TYPE: u8 = 2;
const MUTEX_RELEASE_TYPE: u8 = 3;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MutexPacketType {
    Request,
    Ok,
    Ack,
    Release,
}

pub enum MutexPacket {
    Request(RequestPacket),
    Ok(OkPacket),
    Ack(AckPacket),
    Release(ReleasePacket),
}

impl From<MutexPacket> for Vec<u8> {
    fn from(packet: MutexPacket) -> Self {
        match packet {
            MutexPacket::Request(packet) => packet.into(),
            MutexPacket::Ok(packet) => packet.into(),
            MutexPacket::Ack(packet) => packet.into(),
            MutexPacket::Release(packet) => packet.into(),
        }
    }
}

impl TryFrom<Vec<u8>> for MutexPacket {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(format!(
                "Invalid packet length: expected at least 1, got {}",
                value.len()
            ));
        }

        let packet_type = MutexPacketType::try_from(value[0])?;

        match packet_type {
            MutexPacketType::Request => Ok(MutexPacket::Request(value.try_into()?)),
            MutexPacketType::Ok => Ok(MutexPacket::Ok(value.try_into()?)),
            MutexPacketType::Ack => Ok(MutexPacket::Ack(value.try_into()?)),
            MutexPacketType::Release => Ok(MutexPacket::Release(value.try_into()?)),
        }
    }
}

impl From<MutexPacketType> for u8 {
    fn from(packet_type: MutexPacketType) -> Self {
        match packet_type {
            MutexPacketType::Request => MUTEX_REQUEST_TYPE,
            MutexPacketType::Ack => MUTEX_ACK_TYPE,
            MutexPacketType::Ok => MUTEX_OK_TYPE,
            MutexPacketType::Release => MUTEX_RELEASE_TYPE,
        }
    }
}

impl TryFrom<u8> for MutexPacketType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            MUTEX_REQUEST_TYPE => Ok(MutexPacketType::Request),
            MUTEX_ACK_TYPE => Ok(MutexPacketType::Ack),
            MUTEX_OK_TYPE => Ok(MutexPacketType::Ok),
            MUTEX_RELEASE_TYPE => Ok(MutexPacketType::Release),
            _ => Err(format!("Unknown packet type: {}", value)),
        }
    }
}
