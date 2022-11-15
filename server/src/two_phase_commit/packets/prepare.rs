use crate::two_phase_commit::TransactionId;
use rand;
use rand::Rng;

pub struct PreparePacket {
    pub transaction_id: TransactionId,
    client_id: u32,
    new_value: u32,
}

impl PreparePacket {
    pub fn new(client_id: u32, new_value: u32) -> Self {
        // TODO: Avoid randomness
        let transaction_id = TransactionId {
            id: rand::thread_rng().gen(),
        };

        Self {
            transaction_id,
            client_id,
            new_value,
        }
    }
}

impl From<&[u8]> for PreparePacket {
    fn from(bytes: &[u8]) -> Self {
        let transaction_id = TransactionId::from(&bytes[0..4]);
        
        let client_id = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let new_value = u32::from_be_bytes(bytes[8..12].try_into().unwrap());

        Self {
            transaction_id,
            client_id,
            new_value,
        }
    }
}

impl From<PreparePacket> for Vec<u8> {
    fn from(packet: PreparePacket) -> Self {
        let mut bytes = Vec::new();
        let transaction_id: [u8; 4] = packet.transaction_id.into();
        bytes.extend_from_slice(&transaction_id);
        bytes.extend_from_slice(&packet.client_id.to_be_bytes());
        bytes.extend_from_slice(&packet.new_value.to_be_bytes());

        bytes
    }
}