pub(crate) mod receiver;
pub(crate) mod sender;
use async_trait::async_trait;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("send error: ")]
pub enum SendError<T: Debug> {
    #[error("channel closed, message: {0:?}")]
    Return(T),

    #[error("channel closed")]
    Closed,
}

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
