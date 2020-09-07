use super::{SendError, Sender};
use async_trait::async_trait;
use log::trace;

use std::{fmt::Debug, marker::PhantomData};

/// A wrapper which provides a stable [Sender](./trait.Sender.html) implementation, returned by [bus.tx::\<Msg\>()](trait.Bus.html#tymethod.tx).
/// Can be unwrapped with [into_inner()](./struct.LifelineSender.html#method.into_inner)
pub struct LifelineSender<T, S> {
    inner: S,
    log: bool,
    _t: PhantomData<T>,
}

impl<T, S> LifelineSender<T, S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            log: false,
            _t: PhantomData,
        }
    }

    /// Enables trace-level logging for messages sent over the channel
    pub fn log(mut self) -> Self {
        self.log = true;
        self
    }

    /// Returns a reference to the inner sender
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Returns a mutable reference to the inner sender
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consumes the wrapper, and returns the inner sender
    pub fn into_inner(self) -> S {
        self.inner
    }
}

#[async_trait]
impl<T, S> Sender<T> for LifelineSender<T, S>
where
    T: Send + Debug,
    S: Send + Sender<T>,
{
    async fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        let log = if self.log && log::log_enabled!(log::Level::Trace) {
            Some(format!("SEND {:?}", &value))
        } else {
            None
        };

        let result = self.inner.send(value).await;

        if let Some(log) = log {
            trace!("{}", log);
        }

        result
    }
}

impl<T, S: Debug> Debug for LifelineSender<T, S> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Receiver")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T, S> Clone for LifelineSender<T, S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            log: self.log,
            _t: PhantomData,
        }
    }
}
