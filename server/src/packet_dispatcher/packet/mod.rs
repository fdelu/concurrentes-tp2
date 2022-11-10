const MUTEX_TYPE: u8 = 0;
const COMMIT_TYPE: u8 = 1;

pub enum PacketType {
    Mutex,
    Commit,
}

impl TryFrom<u8> for PacketType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            MUTEX_TYPE => Ok(PacketType::Mutex),
            COMMIT_TYPE => Ok(PacketType::Commit),
            _ => Err(format!("Invalid packet type: {}", value)),
        }
    }
}

impl From<PacketType> for u8 {
    fn from(value: PacketType) -> Self {
        match value {
            PacketType::Mutex => MUTEX_TYPE,
            PacketType::Commit => COMMIT_TYPE,
        }
    }
}
