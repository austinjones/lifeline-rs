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

pub trait ReceiverExt<T>: Receiver<T> + Unpin + Send + Sized {
    fn map<T2, Map>(self, map: Map) -> MapReceiver<Self, T, T2, Map>
    where
        Map: Fn(T) -> T2 + Send + Unpin,
        T: Send + Unpin,
        MapReceiver<Self, T, T2, Map>: ReceiverExt<T2>,
    {
        MapReceiver::new(self, map)
    }

    fn merge<R2>(self, other: R2) -> MergeReceiver<Self, R2, T>
    where
        R2: Receiver<T> + Unpin + Send,
        T: Unpin + Send,
        MergeReceiver<Self, R2, T>: ReceiverExt<T>,
    {
        MergeReceiver::new(self, other)
    }

    fn merge_from<R2, T2, O>(self, other: R2) -> MergeFromReceiver<Self, R2, T, T2>
    where
        R2: Receiver<T2> + Unpin + Send,
        T: From<T2> + Unpin + Send,
        T2: Unpin + Send,
        MergeFromReceiver<Self, R2, T, T2>: ReceiverExt<T2>,
    {
        MergeFromReceiver::new(self, other)
    }
}

impl<R, T> ReceiverExt<T> for R where R: Receiver<T> + Unpin + Send + Sized {}
