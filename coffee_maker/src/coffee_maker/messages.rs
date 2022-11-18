use std::io::Read;

use actix::Message;
use common::packet::TxId;

use super::Coffee;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReadOrdersFrom {
    pub reader: Box<dyn Read + Send + 'static>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct MakeCoffee {
    pub coffee: Coffee,
    pub tx_id: TxId,
}
