use super::Channel;
use crate::error::SendError as LifelineSendError;
use crate::{impl_channel_clone, impl_channel_take};
use async_std::channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use std::fmt::Debug;

impl<T: Send + 'static> Channel for Sender<T> {
    type Tx = Self;
    type Rx = Receiver<T>;

    fn channel(capacity: usize) -> (Self::Tx, Self::Rx) {
        bounded(capacity)
    }

    fn default_capacity() -> usize {
        16
    }
}

impl_channel_clone!(Sender<T>);
impl_channel_take!(Receiver<T>);

#[async_trait]
impl<T> crate::Sender<T> for Sender<T>
where
    T: Debug + Send,
{
    async fn send(&mut self, value: T) -> Result<(), LifelineSendError<T>> {
        Sender::send(self, value)
            .await
            .map_err(|err| LifelineSendError::Return(err.0))?;

        Ok(())
    }
}

#[async_trait]
impl<T> crate::Receiver<T> for Receiver<T>
where
    T: Debug + Send,
{
    async fn recv(&mut self) -> Option<T> {
        Receiver::recv(self).await.ok()
    }
}
