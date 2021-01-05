use async_trait::async_trait;
use futures_util::future::{select, Either};
use std::{marker::PhantomData, pin::Pin};

use crate::Receiver;

pub struct MergeFromReceiver<R1, R2, T, T2>
where
    R1: Receiver<T> + Unpin + Send,
    R2: Receiver<T2> + Unpin + Send,
    T: From<T2> + Unpin + Send,
    T2: Unpin + Send,
{
    r1: R1,
    r2: R2,
    r1_first: bool,
    _t: PhantomData<T>,
    _t2: PhantomData<T2>,
}

impl<R1, R2, T, T2> MergeFromReceiver<R1, R2, T, T2>
where
    R1: Receiver<T> + Unpin + Send,
    R2: Receiver<T2> + Unpin + Send,
    T: From<T2> + Unpin + Send,
    T2: Unpin + Send,
{
    pub fn new(r1: R1, r2: R2) -> Self {
        Self {
            r1,
            r2,
            r1_first: true,
            _t: PhantomData,
            _t2: PhantomData,
        }
    }
}

#[async_trait]
impl<R1, R2, T, T2> Receiver<T> for MergeFromReceiver<R1, R2, T, T2>
where
    R1: Receiver<T> + Unpin + Send,
    R2: Receiver<T2> + Unpin + Send,
    T: From<T2> + Unpin + Send,
    T2: Unpin + Send,
{
    async fn recv(&mut self) -> Option<T> {
        self.r1_first = !self.r1_first;
        let r1_first = self.r1_first;

        let mut r1 = Pin::new(&mut self.r1);
        let mut r2 = Pin::new(&mut self.r2);

        if r1_first {
            let select = select(r1.recv(), r2.recv()).await;

            let x = match select {
                Either::Left((val, _fut)) => val,
                Either::Right((val, _fut)) => val.map(T::from),
            };

            x
        } else {
            let select = select(r2.recv(), r1.recv()).await;

            let x = match select {
                Either::Left((val, _fut)) => val.map(T::from),
                Either::Right((val, _fut)) => val,
            };

            x
        }
    }
}
