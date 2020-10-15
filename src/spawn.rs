use futures_util::task::AtomicWaker;
use std::fmt::Debug;
use std::future::Future;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Poll,
};

use crate::error::type_name;
use log::debug;
use pin_project::pin_project;

/// Executes the task, until the future completes, or the lifeline is dropped
///
/// If the `tokio-executor` feature is enabled, then it is used to spawn the task
///
/// Otherwise, if the `async-std-executor` feature is enabled, then it is used to spawn the task
#[allow(unreachable_code)]
pub(crate) fn spawn_task<O>(name: String, fut: impl Future<Output = O> + Send + 'static) -> Lifeline
where
    O: Debug + Send + 'static,
{
    let inner = Arc::new(LifelineInner::new());

    let service = LifelineFuture::new(name, fut, inner.clone());

    #[cfg(feature = "tokio-executor")]
    {
        spawn_task_tokio(service);
        return Lifeline::new(inner);
    }

    #[cfg(feature = "async-std-executor")]
    {
        spawn_task_async_std(service);
        return Lifeline::new(inner);
    }
}

pub(crate) fn task_name<S>(name: &str) -> String {
    type_name::<S>().to_string() + "/" + name
}

/// Spawns a task using the tokio executor
#[cfg(feature = "tokio-executor")]
fn spawn_task_tokio<F, O>(task: F)
where
    F: Future<Output = O> + Send + 'static,
    O: Send + 'static,
{
    tokio::spawn(task);
}

/// Spawns a task using the async-std executor
#[cfg(feature = "async-std-executor")]
fn spawn_task_async_std<F, O>(task: F)
where
    F: Future<Output = O> + Send + 'static,
    O: Send + 'static,
{
    async_std::task::spawn(task);
}

/// A future which wraps another future, and immediately returns Poll::Ready if the associated lifeline handle has been dropped.
///
/// This is the critical component of the lifeline library, which allows the transparent & immediate cancelleation of entire Service trees.
#[pin_project]
struct LifelineFuture<F: Future> {
    #[pin]
    future: F,
    name: String,
    inner: Arc<LifelineInner>,
}

impl<F: Future + Send> LifelineFuture<F> {
    pub fn new(name: String, future: F, inner: Arc<LifelineInner>) -> Self {
        debug!("START {}", &name);

        Self {
            name,
            future,
            inner,
        }
    }
}

impl<F: Future> Future for LifelineFuture<F>
where
    F::Output: Debug,
{
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.inner.complete.load(Ordering::Relaxed) {
            debug!("CANCEL {}", self.name);
            return Poll::Ready(());
        }

        // attempt to complete the future
        if let Poll::Ready(result) = self.as_mut().project().future.poll(cx) {
            debug!("END {} {:?}", self.name, result);
            return Poll::Ready(());
        }

        // Register to receive a wakeup if the future is aborted in the... future
        self.inner.task_waker.register(cx.waker());

        // Check to see if the future was aborted between the first check and
        // registration.
        // Checking with `Relaxed` is sufficient because `register` introduces an
        // `AcqRel` barrier.
        if self.inner.complete.load(Ordering::Relaxed) {
            debug!("CANCEL {}", self.name);
            return Poll::Ready(());
        }

        Poll::Pending
    }
}

/// A lifeline value, associated with a future spawned via the `Task` trait.  When the lifeline is dropped, the associated future is immediately cancelled.
///
/// Lifeline values can be combined into structs, and represent trees of cancellable tasks.
///
/// Example:
/// ```
/// use lifeline::Task;
/// use lifeline::Lifeline;
///
/// struct ExampleService {}
/// impl ExampleService {
///     fn my_method() -> Lifeline {
///         Self::task("my_method", async move {
///             // some impl
///         })
///     }
/// }
/// ```
#[derive(Debug)]
#[must_use = "if unused the service will immediately be cancelled"]
pub struct Lifeline {
    inner: Arc<LifelineInner>,
}

impl Lifeline {
    pub(crate) fn new(inner: Arc<LifelineInner>) -> Self {
        Self { inner }
    }
}

impl Future for Lifeline {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if self.inner.complete.load(Ordering::Relaxed) {
            return Poll::Ready(());
        }

        // Register to receive a wakeup if the future is aborted in the... future
        self.inner.lifeline_waker.register(cx.waker());

        // Check to see if the future was aborted between the first check and
        // registration.
        // Checking with `Relaxed` is sufficient because `register` introduces an
        // `AcqRel` barrier.
        if self.inner.complete.load(Ordering::Relaxed) {
            return Poll::Ready(());
        }

        Poll::Pending
    }
}

impl Drop for Lifeline {
    fn drop(&mut self) {
        self.inner.abort();
    }
}

#[derive(Debug)]
pub(crate) struct LifelineInner {
    task_waker: AtomicWaker,
    lifeline_waker: AtomicWaker,
    complete: AtomicBool,
}

impl LifelineInner {
    pub fn new() -> Self {
        LifelineInner {
            task_waker: AtomicWaker::new(),
            lifeline_waker: AtomicWaker::new(),
            complete: AtomicBool::new(false),
        }
    }

    pub fn abort(&self) {
        self.complete.store(true, Ordering::Relaxed);
        self.task_waker.wake();
    }
}
