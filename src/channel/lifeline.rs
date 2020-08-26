pub(crate) mod receiver;
pub(crate) mod sender;
use async_trait::async_trait;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("send error: {0:?}")]
pub struct SendError<T: Debug>(pub T);

#[async_trait]
pub trait Sender<T: Debug> {
    async fn send(&mut self, value: T) -> Result<(), SendError<T>>;
}

#[async_trait]
pub trait Receiver<T> {
    async fn recv(&mut self) -> Option<T>;
}
