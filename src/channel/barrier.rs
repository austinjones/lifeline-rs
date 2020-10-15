use arc_swap::ArcSwap;
use async_trait::async_trait;
use lockfree::stack::Stack;
use std::{
    fmt::Debug, future::Future, marker::PhantomData, sync::atomic::AtomicBool,
    sync::atomic::Ordering, sync::Arc, task::Poll, task::Waker,
};

use crate::{Channel, Receiver, Sender, Storage};

pub fn barrier<T: Clone + Default + Sync>() -> (Barrier<T>, BarrierReceiver<T>) {
    let inner = Arc::new(BarrierInner::new());
    let barrier = Barrier::new(inner.clone());
    let receiver = BarrierReceiver::new(inner);

    (barrier, receiver)
}

/// A type which provdides a runtime synchronization barrier.
/// BarrierReceiver implements Future, and the associated receiver completes when this barrier is dropped, or when release is called.
pub struct Barrier<T: Clone + Default + Sync> {
    inner: Arc<BarrierInner<T>>,
    _t: PhantomData<T>,
}

impl<T: Clone + Default + Sync> Barrier<T> {
    pub(in crate::channel::barrier) fn new(inner: Arc<BarrierInner<T>>) -> Self {
        Self {
            inner,
            _t: PhantomData,
        }
    }

    /// Releases the waker early.  
    pub fn release(self, value: T) {
        self.inner.release(value)
    }
}

impl<T: Clone + Default + Sync> Drop for Barrier<T> {
    fn drop(&mut self) {
        self.inner.release(T::default())
    }
}

impl<T: Clone + Default + Sync + 'static> Storage for Barrier<T> {
    fn take_or_clone(res: &mut Option<Self>) -> Option<Self> {
        Self::take_slot(res)
    }
}

#[async_trait]
impl<T: Clone + Debug + Default + Send + Sync> Sender<T> for Barrier<T> {
    async fn send(&mut self, value: T) -> Result<(), crate::error::SendError<T>> {
        self.inner.release(value);

        Ok(())
    }
}

/// An asynchronous receiver for a synchronous barrier.
///
/// Implements Future, and resolves when the associated Barrier sender has been dropped.
pub struct BarrierReceiver<T: Clone + Default + Sync> {
    inner: Arc<BarrierInner<T>>,
    _t: PhantomData<T>,
}

impl<T: Clone + Default + Sync> BarrierReceiver<T> {
    pub(in crate::channel::barrier) fn new(inner: Arc<BarrierInner<T>>) -> Self {
        Self {
            inner,
            _t: PhantomData,
        }
    }

    /// Returns when the associated barrier has been dropped.
    ///
    /// Equivalent to `self.await` or `self.clone().await`
    pub async fn recv(&self) {
        let receiver = self.clone();
        receiver.await;
    }
}

impl<T: Clone + Default + Sync> Future for BarrierReceiver<T> {
    type Output = T;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.inner.released.load(Ordering::Relaxed) {
            return Poll::Ready(self.inner.value());
        }

        self.inner.waker.register(cx.waker());

        if self.inner.released.load(Ordering::Relaxed) {
            return Poll::Ready(self.inner.value());
        }

        Poll::Pending
    }
}

impl<T: Clone + Default + Sync> Clone for BarrierReceiver<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _t: PhantomData,
        }
    }
}

/// Barrier doesn't actually contain a T, and Arc is send
unsafe impl<T: Clone + Default + Sync> Send for BarrierReceiver<T> {}

impl<T: Clone + Default + Sync + 'static> Storage for BarrierReceiver<T> {
    fn take_or_clone(res: &mut Option<Self>) -> Option<Self> {
        Self::clone_slot(res)
    }
}

#[async_trait]
impl<T: Clone + Default + Sync> Receiver<T> for BarrierReceiver<T> {
    async fn recv(&mut self) -> Option<T> {
        let receiver = self.clone();
        receiver.await;

        None
    }
}

struct BarrierWaker {
    wakers: Stack<Waker>,
}

impl BarrierWaker {
    pub fn new() -> Self {
        Self {
            wakers: Stack::new(),
        }
    }

    pub fn register(&self, waker: &Waker) {
        self.wakers.push(waker.clone());
    }

    pub fn wake(&self) {
        for waker in self.wakers.pop_iter() {
            waker.wake();
        }
    }
}

struct BarrierValue<T: Default + Sync> {
    slot: ArcSwap<T>,
}

impl<T: Clone + Default + Sync> BarrierValue<T> {
    pub fn new() -> Self {
        Self {
            slot: ArcSwap::new(Arc::new(T::default())),
        }
    }

    pub fn store(&self, value: T) {
        self.slot.store(Arc::new(value));
    }

    pub fn retrieve(&self) -> T {
        (**self.slot.load()).clone()
    }
}

struct BarrierInner<T: Clone + Default + Sync> {
    released: AtomicBool,
    waker: BarrierWaker,
    value: BarrierValue<T>,
}

impl<T: Clone + Default + Sync> BarrierInner<T> {
    pub fn new() -> Self {
        Self {
            released: AtomicBool::new(false),
            waker: BarrierWaker::new(),
            value: BarrierValue::new(),
        }
    }

    pub fn value(&self) -> T {
        self.value.retrieve()
    }

    pub fn release(&self, value: T) {
        self.value.store(value);
        self.released.store(true, Ordering::Relaxed);
        self.waker.wake();
    }
}

impl<T: Clone + Default + Send + Sync + 'static> Channel for Barrier<T> {
    type Tx = Barrier<T>;
    type Rx = BarrierReceiver<T>;

    fn channel(_capacity: usize) -> (Self::Tx, Self::Rx) {
        barrier()
    }

    fn default_capacity() -> usize {
        0
    }
}
