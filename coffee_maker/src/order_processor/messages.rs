use actix::{Message, Recipient};

use crate::{
    coffee_maker::{Coffee, MakeCoffee},
    order_processor::TransactionResult,
};
use common::{
    packet::{Amount, TxId, UserId},
    AHandler,
};

pub trait OrderProcessorTrait:
    AHandler<RedeemCoffee>
    + AHandler<CommitRedemption>
    + AHandler<AbortRedemption>
    + AHandler<AddPoints>
{
}

///mensaje para comenzar a preparar una orden
#[derive(Message)]
#[rtype(result = "TransactionResult")]
pub struct RedeemCoffee {
    pub coffee: Coffee,
    pub maker: Recipient<MakeCoffee>,
}

///mensaje para oficializar la orden preparada
#[derive(Message)]
#[rtype(result = "()")]
pub struct CommitRedemption {
    pub transaction_id: TxId,
    pub coffee: Coffee,
}

///mensaje para abortar la orden que se estaba preparando
#[derive(Message)]
#[rtype(result = "()")]
pub struct AbortRedemption {
    pub transaction_id: TxId,
    pub coffee: Coffee,
}

///mensaje para agregar dinero a la cuenta de un usuario
#[derive(Message)]
#[rtype(result = "()")]
pub struct AddPoints {
    pub user_id: UserId,
    pub amount: Amount,
}
