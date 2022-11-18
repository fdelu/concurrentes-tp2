use actix::Message;
use common::packet::{Amount, TxId, UserId};

///mensaje para comenzar a preparar una orden
#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct PrepareOrder {
    pub user_id: UserId,
    pub cost: Amount,
}

///mensaje para oficializar la orden preparada
#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct CommitOrder {
    pub transaction_id: TxId,
}

///mensaje para abortar la orden que se estaba preparando
#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AbortOrder {
    pub transaction_id: TxId,
}

///mensaje para agregar dinero a la cuenta de un usuario
#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AddMoney {
    pub user_id: UserId,
    pub amount: Amount,
}
