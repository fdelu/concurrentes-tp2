mod ack;
mod ok;
mod request;

use crate::ResourceId;
pub(crate) use ack::AckPacket;
pub(crate) use ok::OkPacket;
pub(crate) use request::RequestPacket;

const MUTEX_REQUEST_TYPE: u8 = 0;
const MUTEX_ACK_TYPE: u8 = 1;
const MUTEX_OK_TYPE: u8 = 2;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MutexPacketType {
    Request,
    Ok,
    Ack,
}

pub enum MutexPacket {
    Request(RequestPacket),
    Ok(OkPacket),
    Ack(AckPacket),
}

fn decode_mutex_packet_with_only_id(
    packet_bytes: Vec<u8>,
    expected_type: MutexPacketType,
) -> Result<ResourceId, String> {
    if packet_bytes.len() != 5 {
        return Err(format!(
            "Invalid packet length: expected 5, got {}",
            packet_bytes.len()
        ));
    }

    let packet_type = MutexPacketType::try_from(packet_bytes[0])?;
    if packet_type != expected_type {
        return Err(format!(
            "Invalid packet type: expected {:?}, got {:?}",
            expected_type, packet_type
        ));
    }

    let id = packet_bytes[1..5].try_into().unwrap();
    Ok(id)
}

impl From<MutexPacket> for Vec<u8> {
    fn from(packet: MutexPacket) -> Self {
        match packet {
            MutexPacket::Request(packet) => packet.into(),
            MutexPacket::Ok(packet) => packet.into(),
            MutexPacket::Ack(packet) => packet.into(),
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
        }
    }
}

impl From<MutexPacketType> for u8 {
    fn from(packet_type: MutexPacketType) -> Self {
        match packet_type {
            MutexPacketType::Request => MUTEX_REQUEST_TYPE,
            MutexPacketType::Ack => MUTEX_ACK_TYPE,
            MutexPacketType::Ok => MUTEX_OK_TYPE,
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
            _ => Err(format!("Unknown packet type: {}", value)),
        }
    }
}
