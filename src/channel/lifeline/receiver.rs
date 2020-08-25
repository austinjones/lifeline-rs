use super::Receiver;
use async_trait::async_trait;

use pin_project::pin_project;
use std::{
    fmt::Debug,
    marker::{PhantomData, Send},
};

#[pin_project(project = InnerProjection)]
pub struct LifelineReceiver<T, R> {
    #[pin]
    inner: R,
    log: bool,
    // will the channel stop if there are
    strict: bool,
    _t: PhantomData<T>,
}

impl<T, R> LifelineReceiver<T, R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            log: false,
            strict: false,
            _t: PhantomData,
        }
    }

    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    pub fn log(mut self) -> Self {
        self.log = true;
        self
    }

    pub fn inner(&self) -> &R {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

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
        self.inner.recv().await
    }
}

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
