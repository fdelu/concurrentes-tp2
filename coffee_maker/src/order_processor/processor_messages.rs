use actix::Message;
use std::io;

#[derive(Message)]
#[rtype(result = "Result<String, io::Error>")]
pub(crate) struct PrepareOrder {
    pub user_id: u8,
    pub cost: u8,
}

#[derive(Message)]
#[rtype(result = "Result<String, io::Error>")]
pub(crate) struct CommitOrder {}

#[derive(Message)]
#[rtype(result = "Result<String, io::Error>")]
pub(crate) struct AbortOrder {}

#[derive(Message)]
#[rtype(result = "Result<String, io::Error>")]
pub(crate) struct AddMoney {
    pub user_id: u8,
    pub amount: u8,
}
