use std::fmt::{Display, Formatter};

use common::error::CoffeeError;

#[derive(Debug, Clone)]
/// Result of a transaction
pub enum TransactionResult {
    Completed,
    Failed(CoffeeError),
}

impl<E> From<E> for TransactionResult
where
    CoffeeError: From<E>,
{
    fn from(error: E) -> Self {
        TransactionResult::Failed(CoffeeError::from(error))
    }
}

impl Display for TransactionResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionResult::Completed => write!(f, "Completed"),
            TransactionResult::Failed(error) => write!(f, "Failed: {}", error),
        }
    }
}
