use async_trait::async_trait;
use futures_util::future::{select, Either};
use std::{marker::PhantomData, pin::Pin};

use crate::Receiver;

pub struct MergeReceiver<R1, R2, T>
where
    R1: Receiver<T> + Send,
    R2: Receiver<T> + Send,
    T: Send,
{
    r1: R1,
    r2: R2,
    r1_first: bool,
    _t: PhantomData<T>,
}

impl<R1, R2, T> MergeReceiver<R1, R2, T>
where
    R1: Receiver<T> + Send,
    R2: Receiver<T> + Send,
    T: Send,
{
    pub fn new(r1: R1, r2: R2) -> Self {
        Self {
            r1,
            r2,
            r1_first: true,
            _t: PhantomData,
        }
    }
}

#[async_trait]
impl<R1, R2, T> Receiver<T> for MergeReceiver<R1, R2, T>
where
    R1: Receiver<T> + Unpin + Send,
    R2: Receiver<T> + Unpin + Send,
    T: Unpin + Send,
{
    async fn recv(&mut self) -> Option<T> {
        self.r1_first = !self.r1_first;
        let r1_first = self.r1_first;

        let mut r1 = Pin::new(&mut self.r1);
        let mut r2 = Pin::new(&mut self.r2);

        let select = if r1_first {
            select(r1.recv(), r2.recv())
        } else {
            select(r2.recv(), r1.recv())
        };

        let x = match select.await {
            Either::Left((val, _fut)) => val,
            Either::Right((val, _fut)) => val,
        };

        x
    }
}
