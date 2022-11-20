use actix::Message;
use common::packet::TxId;
use tokio::io::AsyncRead;

use super::{order::Order, Coffee};

#[derive(Message)]
#[rtype(result = "()")]
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
#[rtype(result = "()")]
pub struct AddOrder {
    pub order: Order,
}
