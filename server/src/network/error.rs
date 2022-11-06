use std::{
    error::Error,
    fmt::{Display, Formatter},
};

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