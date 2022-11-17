use serde::{Deserialize, Serialize};
use rand::Rng;
use crate::two_phase_commit::TransactionId;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreparePacket {
    pub transaction_id: TransactionId,
    pub transaction: Transaction,
}

impl PreparePacket {
    pub fn new(client_id: u32, new_value: u32) -> Self {
        // TODO: Avoid randomness
        let transaction_id = rand::thread_rng().gen();

        Self {
            transaction_id,
            transaction: Transaction::Block {
                client_id,
                new_value,
            },
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Transaction {
    Block {
        client_id: u32,
        new_value: u32,
    },
    Discount
}