use std::{
    error::Error,
    fmt::{Display, Formatter},
    io,
};

use serde::{Deserialize, Serialize};

use crate::error::FlattenResult;

/// Tipo de error para el [Socket](super::Socket).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocketError {
    message: String,
}

impl SocketError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl<T: Error> From<T> for SocketError {
    fn from(e: T) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

impl Display for SocketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<SocketError> for io::Error {
    fn from(e: SocketError) -> Self {
        Self::new(io::ErrorKind::Other, e.to_string())
    }
}

impl<A, B, T> FlattenResult<T, SocketError> for Result<Result<T, A>, B>
where
    SocketError: From<A> + From<B>,
{
    /// Convierte un [Result] de [Result] en un [Result].
    fn flatten(self) -> Result<T, SocketError> {
        self.map_err(SocketError::from)
            .and_then(|r| r.map_err(SocketError::from))
    }
}
