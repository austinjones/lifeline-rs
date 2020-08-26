pub(crate) mod receiver;
pub(crate) mod sender;
use crate::error::SendError;
use async_trait::async_trait;
use std::fmt::Debug;

impl<T: Debug> SendError<T> {
    pub fn take_message(self) -> Option<T> {
        match self {
            SendError::Return(value) => Some(value),
            SendError::Closed => None,
        }
    }
}

#[async_trait]
pub trait Sender<T: Debug> {
    async fn send(&mut self, value: T) -> Result<(), SendError<T>>;
}

#[async_trait]
pub trait Receiver<T> {
    async fn recv(&mut self) -> Option<T>;
}
