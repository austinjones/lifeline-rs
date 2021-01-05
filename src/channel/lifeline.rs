pub(crate) mod receiver;
pub(crate) mod sender;
use crate::error::SendError;
use async_trait::async_trait;
use std::fmt::Debug;

use super::combinators::{MapReceiver, MergeFromReceiver, MergeReceiver};

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

pub trait ReceiverExt<T>: Send + Sized + Receiver<T> {
    fn map<O, F>(self, map: F) -> MapReceiver<Self, T, O, F>
    where
        F: Send + FnMut(T) -> O,
    {
        MapReceiver::new(self, map)
    }

    fn merge<R>(self, other: R) -> MergeReceiver<Self, R, T>
    where
        R: Receiver<T> + Send,
        T: Send,
    {
        MergeReceiver::new(self, other)
    }

    fn merge_from<R, T2, O>(self, other: R) -> MergeFromReceiver<Self, R, T, T2>
    where
        R: ReceiverExt<T2> + Unpin + Send,
        T: From<T2> + Send,
    {
        MergeFromReceiver::new(self, other)
    }
}

impl<R, T> ReceiverExt<T> for R where R: Receiver<T> + Send {}
