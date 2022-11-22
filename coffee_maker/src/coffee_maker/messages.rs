use actix::Message;
use common::packet::TxId;
use tokio::io::AsyncRead;

use super::{order::Order, Coffee};
use crate::order_processor::TransactionResult;

#[derive(Message)]
#[rtype(result = "Vec<(Order, TransactionResult)>")]
pub struct ReadOrdersFrom<R: AsyncRead> {
    pub reader: R,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct MakeCoffee {
    pub coffee: Coffee,
    pub tx_id: TxId,
}

#[derive(Message)]
#[rtype(result = "TransactionResult")]
pub struct AddOrder {
    pub order: Order,
}
