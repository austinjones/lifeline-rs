use crate::{
    spawn::{spawn_task, task_name},
    Bus, Lifeline,
};
use log::{debug, error};
use std::future::Future;
use std::{any::TypeId, fmt::Debug};

/// Takes channels from the [Bus](./trait.Bus.html), and spawns a tree of tasks.  Returns one or more [Lifeline](./struct.Lifeline.html) values.  
/// When the [Lifeline](./struct.Lifeline.html) is dropped, the task tree is immediately cancelled.
///
/// - Simple implementations can return the [Lifeline](./struct.Lifeline.html) value, a handle returned by [Task::task](./trait.Task.html#method.task).
/// - Implementations which have fallible spawns can return `anyhow::Result<Lifeline>`.
/// - Implementations which spawn multiple tasks can store lifelines for each task in self, and return `anyhow::Result<Self>`.
///
/// ## Example
/// ```
/// use lifeline::prelude::*;
/// use tokio::sync::mpsc;
///
/// lifeline_bus!(pub struct ExampleBus);
///
/// #[derive(Debug, Clone)]
/// struct ExampleMessage {}
///
/// impl Message<ExampleBus> for ExampleMessage {
///     type Channel = mpsc::Sender<Self>;
/// }    
///
/// struct ExampleService {
///     _run: Lifeline   
/// }
///
/// impl Service for ExampleService {
///     type Bus = ExampleBus;
///     type Lifeline = anyhow::Result<Self>;
///
///     fn spawn(bus: &ExampleBus) -> anyhow::Result<Self> {
///         let mut rx = bus.rx::<ExampleMessage>()?;
///
///         let _run = Self::task("run", async move {
///             while let Some(msg) = rx.recv().await {
///                 log::info!("got message: {:?}", msg);
///             }
///         });
///
///         Ok(Self { _run })
///     }
/// }
///
/// async fn run() {
///     let bus = ExampleBus::default();
///     let _service = ExampleService::spawn(&bus);
/// }
/// ```
pub trait Service: Task {
    /// The bus, which must be provided to spawn the task
    type Bus: Bus;

    /// The service lifeline.  When dropped, all spawned tasks are immediately cancelled.
    type Lifeline;

    /// Spawns the service with all sub-tasks, and returns a lifeline value.  When the lifeline is dropped, all spawned tasks are immediately cancelled.
    ///
    /// Implementations should synchronously take channels from the bus, and then use them asynchronously.  This makes errors occur as early and predictably as possible.
    fn spawn(bus: &Self::Bus) -> Self::Lifeline;
}

/// Constructs the bus, spawns the service, and returns both.
pub trait DefaultService: Service {
    fn spawn_default() -> (Self::Bus, Self::Lifeline);
}

impl<T> DefaultService for T
where
    T: Service,
{
    fn spawn_default() -> (Self::Bus, Self::Lifeline) {
        let bus = Self::Bus::default();
        let lifeline = Self::spawn(&bus);

        (bus, lifeline)
    }
}

/// Carries messages between **two** bus instances. A variant of the [Service](./trait.Service.html).
///
/// Bus types form a tree, with a 'root application' bus, and multiple busses focused on particular domains.  This structure provides isolation,
/// and predictable failures when [Services](./trait.Service.html) spawn.
/// ```text
/// - MainBus
///   | ListenerBus
///   |  | ConnectionBus
///   | DomainSpecificBus
///   |  | ...
/// ```
///
/// This trait can be implemented to carry messages between the root and the leaf of the tree.
///
/// ## Example
/// ```
/// use lifeline::prelude::*;
/// use tokio::sync::mpsc;
/// lifeline_bus!(pub struct MainBus);
/// lifeline_bus!(pub struct LeafBus);
///
/// #[derive(Debug, Clone)]
/// struct LeafShutdown {}
///
/// #[derive(Debug, Clone)]
/// struct MainShutdown {}
///
/// impl Message<LeafBus> for LeafShutdown {
///     type Channel = mpsc::Sender<Self>;   
/// }
///
/// impl Message<MainBus> for MainShutdown {
///     type Channel = mpsc::Sender<Self>;   
/// }
///
/// pub struct LeafMainCarrier {
///    _forward_shutdown: Lifeline
/// }
///
/// impl CarryFrom<MainBus> for LeafBus {
///     type Lifeline = anyhow::Result<LeafMainCarrier>;
///     fn carry_from(&self, from: &MainBus) -> Self::Lifeline {
///         let mut rx = self.rx::<LeafShutdown>()?;
///         let mut tx = from.tx::<MainShutdown>()?;
///
///         let _forward_shutdown = Self::try_task("forward_shutdown", async move {
///             if let Some(msg) = rx.recv().await {
///                 tx.send(MainShutdown{}).await?;
///             }
///
///             Ok(())
///         });
///
///         Ok(LeafMainCarrier { _forward_shutdown })
///     }
/// }
/// ```
pub trait CarryFrom<FromBus: Bus>: Bus + Task + Sized {
    /// The carrier lifeline.  When dropped, all spawned tasks are immediately cancelled.
    type Lifeline;

    /// Spawns the carrier service, returning the lifeline value.
    fn carry_from(&self, from: &FromBus) -> Self::Lifeline;
}

/// The receprocial of the [CarryFrom](./trait.CarryFrom.html) trait.  Implemented for all types on which [CarryFrom](./trait.CarryFrom.html) is implemented.
pub trait CarryInto<IntoBus: Bus>: Bus + Task + Sized {
    /// The carrier lifeline.  When dropped, all spawned tasks are immediately cancelled.
    type Lifeline;

    /// Spawns the carrier service, returning the lifeline value.
    fn carry_into(&self, into: &IntoBus) -> Self::Lifeline;
}

impl<F, I> CarryInto<I> for F
where
    I: CarryFrom<F>,
    F: Bus,
    I: Bus,
{
    type Lifeline = <I as CarryFrom<F>>::Lifeline;

    fn carry_into(&self, into: &I) -> Self::Lifeline {
        into.carry_from(self)
    }
}

/// Constructs two bus types, and spawns the carrier between them.
/// Returns both busses, and the carrier's lifeline.
pub trait DefaultCarrier<FromBus: Bus>: CarryFrom<FromBus> {
    fn carry_default() -> (Self, FromBus, Self::Lifeline) {
        let into = Self::default();
        let from = FromBus::default();
        let lifeline = into.carry_from(&from);

        (into, from, lifeline)
    }
}

/// Provides the [Self::task](./trait.Task.html#method.task) and [Self::try_task](./trait.Task.html#method.try_task) associated methods for all types.
///
/// Lifeline supports the following task executors (using feature flags), and will use the first enabled flag:
/// - `tokio-executor`
/// - `async-std-executor`
///
/// Fallible tasks can be invoked with [Self::try_task](./trait.Task.html#method.try_task).  Lifeline will log OK/ERR status when the task finishes.
///
/// # Example
/// ```
/// use lifeline::prelude::*;
/// use tokio::sync::mpsc;
///
/// lifeline_bus!(pub struct ExampleBus);
///
/// #[derive(Debug, Clone)]
/// struct ExampleMessage {}
///
/// impl Message<ExampleBus> for ExampleMessage {
///     type Channel = mpsc::Sender<Self>;
/// }    
///
/// struct ExampleService {
///     _run: Lifeline   
/// }
///
/// impl Service for ExampleService {
///     type Bus = ExampleBus;
///     type Lifeline = anyhow::Result<Self>;
///
///     fn spawn(bus: &ExampleBus) -> anyhow::Result<Self> {
///         let mut rx = bus.rx::<ExampleMessage>()?;
///
///         let _run = Self::task("run", async move {
///             while let Some(msg) = rx.recv().await {
///                 log::info!("got message: {:?}", msg);
///             }
///         });
///
///         Ok(Self { _run })
///     }
/// }
/// ```
pub trait Task {
    /// Spawns an infallible task using the provided executor, wrapping it in a [Lifeline](./struct.Lifeline.html) handle.
    /// The task will run until it finishes, or until the [Lifeline](./struct.Lifeline.html) is droped.
    fn task<Out>(name: &str, fut: impl Future<Output = Out> + Send + 'static) -> Lifeline
    where
        Out: Debug + Send + 'static,
        Self: Sized,
    {
        let service_name = task_name::<Self>(name);
        spawn_task(service_name, fut)
    }

    /// Spawns an fallible task using the provided executor, wrapping it in a [Lifeline](./struct.Lifeline.html) handle.
    /// The task will run until it finishes, or until the [Lifeline](./struct.Lifeline.html) is droped.
    ///
    /// If the task finishes, lifeline will log an 'OK' or 'ERR' message with the return value.
    fn try_task<Out>(
        name: &str,
        fut: impl Future<Output = anyhow::Result<Out>> + Send + 'static,
    ) -> Lifeline
    where
        Out: Debug + 'static,
        Self: Sized,
    {
        let service_name = task_name::<Self>(name);
        spawn_task(service_name.clone(), async move {
            match fut.await {
                Ok(val) => {
                    if TypeId::of::<Out>() != TypeId::of::<()>() {
                        debug!("OK {}: {:?}", service_name, val);
                    } else {
                        debug!("OK {}", service_name);
                    }
                }
                Err(err) => {
                    error!("ERR: {}: {}", service_name, err);
                }
            }
        })
    }
}

impl<T> Task for T {}

// #[async_trait]
// pub trait AsyncService: Task {
//     type Bus: Bus;
//     type Lifeline;

//     async fn spawn(bus: &Self::Bus) -> Self::Lifeline;
// }

// #[async_trait]
// pub trait AsyncCarrier: Task {
//     type From: Bus + Send + Sync + 'static;
//     type Into: Bus + Send + Sync + 'static;
//     type Lifeline;

//     async fn carry(from_bus: &Self::From, to_bus: &Self::Into) -> Self::Lifeline;

//     async fn carry_bus() -> (Self::From, Self::Into, Self::Lifeline) {
//         let from = Self::From::default();
//         let into = Self::Into::default();
//         let lifeline = Self::carry(&from, &into).await;

//         (from, into, lifeline)
//     }
// }
