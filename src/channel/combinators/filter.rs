use async_trait::async_trait;
use std::{marker::PhantomData, pin::Pin};

use crate::Receiver;

pub struct FilterReceiver<R, T, Filter>
where
    R: Receiver<T> + Send + Unpin,
    Filter: Fn(&T) -> bool + Send + Unpin,
    T: Send + Unpin,
{
    inner: R,
    filter: Filter,
    _t: PhantomData<T>,
}

impl<R, T, Filter> FilterReceiver<R, T, Filter>
where
    R: Receiver<T> + Send + Unpin,
    Filter: Fn(&T) -> bool + Send + Unpin,
    T: Send + Unpin,
{
    pub fn new(inner: R, filter: Filter) -> Self {
        Self {
            inner,
            filter,
            _t: PhantomData,
        }
    }
}

#[async_trait]
impl<R, T, Filter> Receiver<T> for FilterReceiver<R, T, Filter>
where
    R: Receiver<T> + Send + Unpin,
    Filter: Fn(&T) -> bool + Send + Unpin,
    T: Send + Unpin,
{
    async fn recv(&mut self) -> Option<T> {
        let mut pin = Pin::new(self);

        loop {
            match pin.inner.recv().await {
                Some(t) => {
                    if (pin.filter)(&t) {
                        return Some(t);
                    }
                }
                None => return None,
            }
        }
    }
}
