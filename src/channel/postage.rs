use super::Channel;
use crate::{error::SendError as LifelineSendError, impl_storage_clone, impl_storage_take};
use crate::{impl_channel_clone, impl_channel_take};
use async_trait::async_trait;
use postage::sink::Sink;
use postage::stream::Stream;
use postage::{barrier, broadcast, dispatch, mpsc, oneshot, watch};
use std::fmt::Debug;

// barrier
impl Channel for barrier::Sender {
    type Tx = Self;
    type Rx = barrier::Receiver;

    fn channel(_capacity: usize) -> (Self::Tx, Self::Rx) {
        barrier::channel()
    }

    fn default_capacity() -> usize {
        1
    }
}

impl_storage_take!(barrier::Sender);
impl_storage_clone!(barrier::Receiver);

// broadcast
impl<T: Clone + Send + 'static> Channel for broadcast::Sender<T> {
    type Tx = Self;
    type Rx = broadcast::Receiver<T>;

    fn channel(capacity: usize) -> (Self::Tx, Self::Rx) {
        broadcast::channel(capacity)
    }

    fn default_capacity() -> usize {
        16
    }

    fn clone_rx(rx: &mut Option<Self::Rx>, tx: Option<&Self::Tx>) -> Option<Self::Rx> {
        rx.take().or_else(|| tx.map(|tx| tx.subscribe()))
    }
}

impl_channel_clone!(broadcast::Sender<T>);
impl_channel_take!(broadcast::Receiver<T>);

#[async_trait]
impl<T> crate::Sender<T> for broadcast::Sender<T>
where
    T: Clone + Debug + Send,
{
    async fn send(&mut self, value: T) -> Result<(), LifelineSendError<T>> {
        Sink::send(self, value)
            .await
            .map(|_| ())
            .map_err(|err| LifelineSendError::Return(err.0))
    }
}

#[async_trait]
impl<T> crate::Receiver<T> for broadcast::Receiver<T>
where
    T: Clone + Debug + Send,
{
    async fn recv(&mut self) -> Option<T> {
        Stream::recv(self).await
    }
}

// mpsc
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
    async fn send(&mut self, value: T) -> Result<(), LifelineSendError<T>> {
        Sink::send(self, value)
            .await
            .map_err(|err| LifelineSendError::Return(err.0))
    }
}

#[async_trait]
impl<T> crate::Receiver<T> for mpsc::Receiver<T>
where
    T: Debug + Send,
{
    async fn recv(&mut self) -> Option<T> {
        Stream::recv(self).await
    }
}

// dispatch
impl<T: Send + 'static> Channel for dispatch::Sender<T> {
    type Tx = Self;
    type Rx = dispatch::Receiver<T>;

    fn channel(capacity: usize) -> (Self::Tx, Self::Rx) {
        dispatch::channel(capacity)
    }

    fn default_capacity() -> usize {
        16
    }
}

impl_channel_clone!(dispatch::Sender<T>);
impl_channel_clone!(dispatch::Receiver<T>);

#[async_trait]
impl<T> crate::Sender<T> for dispatch::Sender<T>
where
    T: Debug + Send,
{
    async fn send(&mut self, value: T) -> Result<(), LifelineSendError<T>> {
        Sink::send(self, value)
            .await
            .map_err(|err| LifelineSendError::Return(err.0))
    }
}

#[async_trait]
impl<T> crate::Receiver<T> for dispatch::Receiver<T>
where
    T: Debug + Send,
{
    async fn recv(&mut self) -> Option<T> {
        Stream::recv(self).await
    }
}

// oneshot
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
        watch::channel()
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
    T: Clone + Debug + Send + Sync,
{
    async fn send(&mut self, value: T) -> Result<(), LifelineSendError<T>> {
        Sink::send(self, value)
            .await
            .map_err(|_| LifelineSendError::Closed)
    }
}

#[async_trait]
impl<T> crate::Receiver<T> for watch::Receiver<T>
where
    T: Clone + Debug + Send + Sync,
{
    async fn recv(&mut self) -> Option<T> {
        Stream::recv(self).await
    }
}
