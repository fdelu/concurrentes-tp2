use core::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    io,
};

use serde::{Deserialize, Serialize};

use crate::socket::SocketError;

/// Tipo de error genérico
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoffeeError {
    InvalidOrder(String),
    InsufficientPoints,
    SocketError(SocketError),
    Other(String),
}

impl CoffeeError {
    pub fn new(message: &str) -> Self {
        Self::Other(message.to_string())
    }
}

impl<T: Error> From<T> for CoffeeError {
    fn from(e: T) -> Self {
        Self::new(&e.to_string())
    }
}

impl From<SocketError> for CoffeeError {
    fn from(e: SocketError) -> Self {
        Self::SocketError(e)
    }
}

impl Display for CoffeeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<CoffeeError> for io::Error {
    fn from(e: CoffeeError) -> Self {
        Self::new(io::ErrorKind::Other, e.to_string())
    }
}

pub trait FlattenResult<T, E> {
    fn flatten(self) -> Result<T, E>;
}

impl<A, B, T> FlattenResult<T, CoffeeError> for Result<Result<T, A>, B>
where
    CoffeeError: From<A> + From<B>,
{
    /// Convierte un [Result] de [Result] en un [Result].
    fn flatten(self) -> Result<T, CoffeeError> {
        self.map_err(CoffeeError::from)
            .and_then(|r| r.map_err(CoffeeError::from))
    }
}
