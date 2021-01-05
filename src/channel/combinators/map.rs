use async_trait::async_trait;
use std::{marker::PhantomData, pin::Pin};

use crate::Receiver;

pub struct MapReceiver<R, T, O, Fn>
where
    R: Receiver<T> + Send,
    Fn: FnMut(T) -> O + Send,
{
    inner: R,
    map: Fn,
    _t: PhantomData<T>,
}

impl<R, T, O, Fn> MapReceiver<R, T, O, Fn>
where
    R: Receiver<T> + Send,
    Fn: FnMut(T) -> O + Send,
{
    pub fn new(inner: R, map: Fn) -> Self {
        Self {
            inner,
            map,
            _t: PhantomData,
        }
    }
}

#[async_trait]
impl<R, T, O, Map> Receiver<O> for MapReceiver<R, T, O, Map>
where
    R: Receiver<T> + Send + Unpin,
    Map: Fn(T) -> O + Send + Unpin,
    O: Default,
    T: Send + Unpin,
{
    async fn recv(&mut self) -> Option<O> {
        let mut pin = Pin::new(self);

        let value = pin.inner.recv().await;
        value.map(|v| (pin.map)(v))
    }
}
