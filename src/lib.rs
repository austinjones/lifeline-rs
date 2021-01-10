//! Lifeline is a dependency injection library for message-based applications.  Lifeline produces applications which are:
//! - **Clean:** Bus implementations provide a high-level overview of the application, and services clearly define the messages they send and receive.
//! - **Decoupled:** Services and tasks have no dependency on their peers, as they only depend on the message types they are sending or receiving.
//! - **Stoppable:** Services and tasks are trivially cancellable.  For example, you can terminate all tasks associated with a connection when a client disconnects.
//! - **Greppable:** The impact/reach of a message can be easily understood by searching for the type in the source code.
//! - **Testable:**  Lifeline applications communicate via messages, which makes unit testing easy.  Spawn the service, send a message, and expect a received message.
//!
//! In order to achieve these goals, lifeline provides patterns, traits, and implementations:
//! - The [Bus](./trait.Bus.html), which constructs & distributes channel Senders/Receivers, and Resources.
//! - The [Carrier](./trait.CarryFrom.html), which translates messages between two Bus instances.  Carriers are critical when building large applications, and help minimize the complexity of the messages on each bus.
//! - The [Service](./trait.Service.html), which takes channels from the bus, and spawns tasks which send and receive messages.
//! - The [Task](./trait.Task.html), an async future which returns a lifeline when spawned. When the lifeline is dropped, the future is immedately cancelled.
//! - The [Resource](./trait.Resource.html), a struct which can be stored in the bus, and taken (or cloned) when services spawn.
//!
//! For a quick introduction, see the [hello.rs example.](https://github.com/austinjones/lifeline-rs/blob/master/examples/hello.rs)
//! For a full-scale application see [tab-rs.](https://github.com/austinjones/tab-rs)
//!
//! ## Quickstart
//! Lifeline uses `tokio` as it's default runtime.  Tokio provides a rich set of async channels.
//! ```toml
//! lifeline = "0.3"
//! ```
//!
//! Lifeline also supports the async-std runtime, and it's `mpsc` channel implementation:
//! ```toml
//! lifeline = { version = "0.3", features = ["dyn-bus", "async-std-executor", "async-std-channels"] }
//! ```
//!
//! ## The Bus
//! The [Bus](./trait.Bus.html) carries channels and resources, and allows you to write loosely coupled [Service](./trait.Service.html) implementations which communicate over messages.
//!
//! Channels can be taken from the bus. If the channel endpoint is clonable, it will remain available for other services.  
//! If the channel is not clonable, future calls will receive an `Err` value. The Rx/Tx type parameters are type-safe,
//! and will produce a compile error if you attempt to take a channel for an message type which the bus does not carry.
//!
//! Lifeline provides a [lifeline_bus!](macro.lifeline_bus.html) macro which stores channels and resources in `Box<dyn>` slots:
//! ```
//! use lifeline::lifeline_bus;
//! lifeline_bus!(pub struct MainBus);
//! ```
//!
//! ## The Carrier
//! [Carriers](./trait.CarryFrom.html) provide a way to move messages between busses. [Carriers](./trait.CarryFrom.html) can translate, ignore, or collect information,
//! providing each bus with the messages that it needs.
//!
//! Large applications have a tree of Busses. This is good, it breaks your app into small chunks.
//! ```text
//! - MainBus
//!   | ConnectionListenerBus
//!   |  | ConnectionBus
//!   | DomainSpecificBus
//!   |  | ...
//! ```
//! [Carriers](./trait.CarryFrom.html) allow each bus to define messages that minimally represent the information it's services need to function, and prevent an explosion of messages which are copied to all busses.
//!
//! [Carriers](./trait.CarryFrom.html) centralize the communication between busses, making large applications easier to reason about.
//!
//! ## The Service
//! The [Service](./trait.Service.html) synchronously takes channels from the [Bus](./trait.Bus.html), and spawns a tree of async tasks (which send & receive messages).
//! When spawned, the service returns one or more [Lifeline](./struct.Lifeline.html) values.  When a [Lifeline](./struct.Lifeline.html) is dropped, the associated task is immediately cancelled.
//!
//! It's common for [Service::spawn](./trait.Service.html#tymethod.spawn) to return a Result.  Taking channel endpoints is a fallible operation.  Depending on the channel type, the endpoint may not be clonable.
//! Lifeline clones endpoints when it can (e.g. for `mpsc::Sender`, `broadcast::*`, and `watch::Receiver`).  Other endpoints are taken, removed, and future calls will return an Err.
//!
//! [Service::spawn](./trait.Service.html#tymethod.spawn) takes channels from the bus synchronously, which makes errors occur predictably and early. If you get an Err on an `mpsc::Receiver`,
//! change it's binding in the bus to `broadcast::Sender`.
//!
//! ## The Task
//! The [Task](./trait.Task.html) executes an Future, and returns a [Lifeline](./struct.Lifeline.html) when spawned.  When the lifeline is dropped, the future is immediately cancelled.
//!
//! [Task](./trait.Task.html) trait is implemented for all types - you can import it and use `Self::task` in any type.  In lifeline, it's
//! most commonly used in Service implementations.
//!
//! ## The Resource
//! [Resources](./trait.Resource.html) can be stored on the bus. This is very useful for configuration (e.g MainConfig), or connections (e.g. a TcpStream).
//!
//! [Resources](./trait.Resource.html) implement the [Storage](./trait.Storage.html) trait, which is easy with the [impl_storage_clone!](./macro.impl_storage_clone.html) and [impl_storage_take!](./macro.impl_storage_take.html) macros.

mod bus;
mod channel;

#[cfg(feature = "dyn-bus")]
pub mod dyn_bus;

pub mod error;
pub mod prelude;

#[cfg(feature = "tokio-channels")]
pub mod request;

mod service;
mod spawn;
mod storage;

// TODO: try to get this as cfg(test)
pub mod test;

pub use bus::*;
pub use channel::lifeline::{Receiver, Sender};

pub use channel::Channel;
pub use service::*;
pub use storage::Storage;
pub use storage::*;

pub use spawn::Lifeline;
