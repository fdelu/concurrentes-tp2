use core::fmt;
use std::fmt::{Display, Formatter};

use actix::MailboxError;
use common::error::CoffeeError;

use crate::two_phase_commit::CommitError;

#[derive(Debug)]
pub enum PacketDispatcherError {
    Timeout,
    InsufficientPoints,
    DiscountFailed,
    IncreaseFailed,
    ActixMailboxFull,
    ServerDisconnected,
    Other,
}

pub type PacketDispatcherResult<T> = Result<T, PacketDispatcherError>;

impl Display for PacketDispatcherError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<PacketDispatcherError> for CoffeeError {
    fn from(e: PacketDispatcherError) -> Self {
        match e {
            PacketDispatcherError::InsufficientPoints => CoffeeError::InsufficientPoints,
            _ => Self::new(&e.to_string()),
        }
    }
}

impl From<MailboxError> for PacketDispatcherError {
    fn from(_: MailboxError) -> Self {
        PacketDispatcherError::ActixMailboxFull
    }
}

impl From<CommitError> for PacketDispatcherError {
    fn from(e: CommitError) -> Self {
        match e {
            CommitError::Timeout => PacketDispatcherError::Timeout,
            CommitError::Disconnected => PacketDispatcherError::ServerDisconnected,
        }
    }
}
