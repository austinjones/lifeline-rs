use super::Receiver;
use async_trait::async_trait;

use log::debug;
use pin_project::pin_project;
use std::{
    fmt::Debug,
    marker::{PhantomData, Send},
};

/// A wrapper which provides a stable [Receiver](./trait.Receiver.html) implementation, returned by [bus.rx::\<Msg\>()](trait.Bus.html#tymethod.rx).
/// Can be unwrapped with [into_inner()](./struct.LifelineReceiver.html#method.into_inner)
#[pin_project(project = InnerProjection)]
pub struct LifelineReceiver<T, R> {
    #[pin]
    inner: R,
    log: bool,
    _t: PhantomData<T>,
}

impl<T, R> LifelineReceiver<T, R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            log: false,
            _t: PhantomData,
        }
    }

    /// Enables trace-level logging for this receiver
    pub fn log(mut self) -> Self {
        self.log = true;
        self
    }

    /// Returns a reference to the inner receiver
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the inner receiver
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes the wrapper, and returns the inner receiver
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<T, R: Debug> Debug for LifelineReceiver<T, R> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Receiver")
            .field("inner", &self.inner)
            .finish()
    }
}

#[async_trait]
impl<T, R> Receiver<T> for LifelineReceiver<T, R>
where
    T: Send + Debug,
    R: Send + Receiver<T>,
{
    async fn recv(&mut self) -> Option<T> {
        let option = self.inner.recv().await;

        if self.log && option.is_some() {
            debug!("RECV: {:?}", option.as_ref().unwrap());
        }

        option
    }
}

impl<T, R> Clone for LifelineReceiver<T, R>
where
    R: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            log: self.log,
            _t: PhantomData,
        }
    }
}

#[cfg(feature = "tokio-channels")]
mod tokio {
    use super::LifelineReceiver;
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };
    use tokio::stream::Stream;

    impl<T, R> Stream for LifelineReceiver<T, R>
    where
        R: Stream<Item = T>,
    {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let project = self.project();
            project.inner.poll_next(cx)
        }
    }
}
