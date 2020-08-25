use super::{SendError, Sender};
use async_trait::async_trait;
use log::trace;
use pin_project::pin_project;
use std::{fmt::Debug, marker::PhantomData};
use thiserror::Error;

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

    pub fn log(mut self) -> Self {
        self.log = true;
        self
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

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
