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

/// The sender half of an asynchronous channel, which may be bounded/unbounded, mpsc/broadcast/oneshot, etc.
///
/// This trait provides a consistent interface for all async senders, which makes your app code
/// very robust to channel changes on the bus.  It also allows `impl Sender<ExampleMessage>` in your associated function signatures.
#[async_trait]
pub trait Sender<T: Debug> {
    async fn send(&mut self, value: T) -> Result<(), SendError<T>>;
}

/// The receiver half of an asynchronous channel, which may be bounded/unbounded, mpsc/broadcast/oneshot, etc.
///
/// This trait provides a consistent interface for all async receivers, which makes your app code
/// very robust to channel changes on the bus.  It also allows `impl Receiver<ExampleMessage>` in your associated function signatures.
#[async_trait]
pub trait Receiver<T> {
    async fn recv(&mut self) -> Option<T>;
}
