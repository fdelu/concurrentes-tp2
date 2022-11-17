use actix::Message;
use std::io;

///mensaje para comenzar a preparar una orden
#[derive(Message)]
#[rtype(result = "Result<String, io::Error>")]
pub(crate) struct PrepareOrder {
    pub user_id: u8,
    pub cost: u8,
}

///mensaje para oficializar la orden preparada
#[derive(Message)]
#[rtype(result = "Result<String, io::Error>")]
pub(crate) struct CommitOrder {}

///mensaje para abortar la orden que se estaba preparando
#[derive(Message)]
#[rtype(result = "Result<String, io::Error>")]
pub(crate) struct AbortOrder {}

///mensaje para agregar dinero a la cuenta de un usuario
#[derive(Message)]
#[rtype(result = "Result<String, io::Error>")]
pub(crate) struct AddMoney {
    pub user_id: u8,
    pub amount: u8,
}
