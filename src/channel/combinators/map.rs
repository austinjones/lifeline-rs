use async_trait::async_trait;
use std::{marker::PhantomData, pin::Pin};

use crate::Receiver;

pub struct MapReceiver<R, T, T2, Map>
where
    R: Receiver<T> + Send + Unpin,
    Map: Fn(T) -> T2 + Send + Unpin,
    T: Send + Unpin,
{
    inner: R,
    map: Map,
    _t: PhantomData<T>,
}

impl<R, T, T2, Map> MapReceiver<R, T, T2, Map>
where
    R: Receiver<T> + Send + Unpin,
    Map: Fn(T) -> T2 + Send + Unpin,
    T: Send + Unpin,
{
    pub fn new(inner: R, map: Map) -> Self {
        Self {
            inner,
            map,
            _t: PhantomData,
        }
    }
}

#[async_trait]
impl<R, T, T2, Map> Receiver<T2> for MapReceiver<R, T, T2, Map>
where
    R: Receiver<T> + Send + Unpin,
    Map: Fn(T) -> T2 + Send + Unpin,
    T: Send + Unpin,
{
    async fn recv(&mut self) -> Option<T2> {
        let mut pin = Pin::new(self);

        let value = pin.inner.recv().await;
        value.map(|v| (pin.map)(v))
    }
}
