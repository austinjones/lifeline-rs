use super::Channel;
use crate::channel::lifeline::SendError as LifelineSendError;
use crate::{error::type_name, impl_channel_clone, impl_channel_take};
use async_trait::async_trait;
use log::debug;
use std::fmt::Debug;
use tokio::sync::{broadcast, mpsc, oneshot, watch};

impl<T: Send + 'static> Channel for mpsc::Sender<T> {
    type Tx = Self;
    type Rx = mpsc::Receiver<T>;

    fn channel(capacity: usize) -> (Self::Tx, Self::Rx) {
        mpsc::channel(capacity)
    }

    fn default_capacity() -> usize {
        16
    }
}

impl_channel_clone!(mpsc::Sender<T>);
impl_channel_take!(mpsc::Receiver<T>);

#[async_trait]
impl<T> crate::Sender<T> for mpsc::Sender<T>
where
    T: Debug + Send,
{
    async fn send(&mut self, value: T) -> Result<(), super::lifeline::SendError<T>> {
        mpsc::Sender::send(self, value)
            .await
            .map_err(|err| LifelineSendError(err.0))
    }
}

#[async_trait]
impl<T> crate::Receiver<T> for mpsc::Receiver<T>
where
    T: Debug + Send,
{
    async fn recv(&mut self) -> Option<T> {
        mpsc::Receiver::recv(self).await
    }
}

impl<T: Send + 'static> Channel for broadcast::Sender<T> {
    type Tx = Self;
    type Rx = broadcast::Receiver<T>;

    fn channel(capacity: usize) -> (Self::Tx, Self::Rx) {
        broadcast::channel(capacity)
    }

    fn default_capacity() -> usize {
        16
    }

    fn clone_rx(rx: &mut Option<Self::Rx>, tx: Option<&Self::Tx>) -> Option<Self::Rx> {
        // tokio channels have a size-limited queue
        // if one receiver stops processing messages,
        // the senders block

        // we take from rx first, getting the bus out of the way
        // then we subscribe using the sender
        // tx should always be here, but just in case.. tx.map( ... )
        rx.take().or_else(|| tx.map(|tx| tx.subscribe()))
    }
}

impl_channel_clone!(broadcast::Sender<T>);

// this is actually overriden in clone_rx
impl_channel_take!(broadcast::Receiver<T>);

#[async_trait]
impl<T> crate::Sender<T> for broadcast::Sender<T>
where
    T: Debug + Send,
{
    async fn send(&mut self, value: T) -> Result<(), super::lifeline::SendError<T>> {
        broadcast::Sender::send(self, value)
            .map(|_| ())
            .map_err(|err| LifelineSendError(err.0))
    }
}

#[async_trait]
impl<T> crate::Receiver<T> for broadcast::Receiver<T>
where
    T: Clone + Debug + Send,
{
    async fn recv(&mut self) -> Option<T> {
        loop {
            let result = broadcast::Receiver::recv(self).await;

            match result {
                Ok(t) => return Some(t),
                Err(broadcast::RecvError::Closed) => return None,
                Err(broadcast::RecvError::Lagged(n)) => {
                    // we keep the broadcast complexity localized here.
                    // instead of making things very complicated for mpsc, watch, etc receivers,
                    // we log a debug message when a lag occurs, even if logging was not requested.

                    debug!("LAGGED {} {}", n, type_name::<T>());
                    continue;
                }
            }
        }
    }
}

impl<T: Send + 'static> Channel for oneshot::Sender<T> {
    type Tx = Self;
    type Rx = oneshot::Receiver<T>;

    fn channel(_capacity: usize) -> (Self::Tx, Self::Rx) {
        oneshot::channel()
    }

    fn default_capacity() -> usize {
        1
    }
}

impl_channel_take!(oneshot::Sender<T>);
impl_channel_take!(oneshot::Receiver<T>);

impl<T> Channel for watch::Sender<T>
where
    T: Default + Clone + Send + Sync + 'static,
{
    type Tx = Self;
    type Rx = watch::Receiver<T>;

    fn channel(_capacity: usize) -> (Self::Tx, Self::Rx) {
        watch::channel(T::default())
    }

    fn default_capacity() -> usize {
        1
    }
}

impl_channel_take!(watch::Sender<T>);
impl_channel_clone!(watch::Receiver<T>);

#[async_trait]
impl<T> crate::Sender<T> for watch::Sender<T>
where
    T: Debug + Send + Sync,
{
    async fn send(&mut self, value: T) -> Result<(), super::lifeline::SendError<T>> {
        watch::Sender::send(self, value)
            .await
            .map_err(|err| LifelineSendError(err.0))
    }
}

#[async_trait]
impl<T> crate::Receiver<T> for watch::Receiver<T>
where
    T: Clone + Debug + Send + Sync,
{
    async fn recv(&mut self) -> Option<T> {
        watch::Receiver::recv(self).await
    }
}
