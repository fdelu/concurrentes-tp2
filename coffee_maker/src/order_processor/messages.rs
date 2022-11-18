use actix::{Message, Recipient};

use crate::coffee_maker::{Coffee, MakeCoffee};
use common::{
    packet::{Amount, TxId, UserId},
    AHandler,
};

pub trait OrderProcessorTrait:
    AHandler<PrepareOrder> + AHandler<CommitOrder> + AHandler<AbortOrder> + AHandler<AddMoney>
{
}

///mensaje para comenzar a preparar una orden
#[derive(Message)]
#[rtype(result = "()")]
pub struct PrepareOrder {
    pub coffee: Coffee,
    pub maker: Recipient<MakeCoffee>,
}

///mensaje para oficializar la orden preparada
#[derive(Message)]
#[rtype(result = "()")]
pub struct CommitOrder {
    pub transaction_id: TxId,
    pub coffee: Coffee,
}

///mensaje para abortar la orden que se estaba preparando
#[derive(Message)]
#[rtype(result = "()")]
pub struct AbortOrder {
    pub transaction_id: TxId,
    pub coffee: Coffee,
}

///mensaje para agregar dinero a la cuenta de un usuario
#[derive(Message)]
#[rtype(result = "()")]
pub struct AddMoney {
    pub user_id: UserId,
    pub amount: Amount,
}
