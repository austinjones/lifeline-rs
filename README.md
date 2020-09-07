# lifeline-rs
Lifeline is a dependency injection library for message-based applications.  Lifeline produces applications which are:
 - **Clean:** Bus implementations provide a high-level overview of the application, and services clearly define the messages they send and receive.
 - **Decoupled:**  Services and tasks have no dependency on their peers, as they only depend on the message types they are sending or receiving.
 - **Stoppable:** Services and tasks are trivially cancellable.  For example, you can terminate all tasks associated with a connection when a client disconnects.
 - **Greppable:**  The impact/reach of a message can be easily understood by searching for the type in the source code.
 - **Testable:**  Lifeline applications communicate via messages, which makes unit testing easy.  Create the bus, spawn the service, send a message, and expect an output message.

In order to achieve these goals, lifeline provides patterns, traits, and implementations:
 - The Bus, which constructs & distributes channel Senders/Receivers, and Resources.
 - The Carrier, which translates messages between two Bus instances.  Carriers are critical when building large applications, and help minimize the complexity of the messages on each bus.
 - The Service, which takes channels from the bus, and spawns tasks which send and receive messages.
 - The Task, an async future which returns a lifeline when spawned.  When the lifeline is dropped, the future is immedately cancelled.
 - The Resource, a struct which can be stored in the bus, and taken (or cloned) when services spawn.

For a quick overview, see the [hello.rs example.](https://github.com/austinjones/lifeline-rs/blob/master/examples/hello.rs).
For a full-scale application see [tab-rs.](https://github.com/austinjones/tab-rs)

## Quickstart
Lifeline uses `tokio` as it's default runtime.  Tokio provides a rich set of async channels.
```toml
lifeline = "0.3"
```

Lifeline also supports the async-std runtime, and it's `mpsc` channel implementation:
```toml
lifeline = { version = "0.3", features = ["dyn-bus", "async-std-executor", "async-std-channels"] }
```

## The Bus
The bus carries channels and resources.  When services spawn, they receive a reference to the bus.

Channels can be taken from the bus.  If the channel endpoint is clonable, it will remain available for other services.  
If the channel is not clonable, future calls will receive an `Err` value.  The Rx/Tx type parameters are type-safe, and will produce a compile error
if you attempt to take a channel for an message type which the bus does not carry.

```rust
lifeline_bus!(pub struct MainBus);

let rx = bus.rx::<MessageType>()?;
```

Here is a full example where:
- a bus is constructed, 
- a sender is taken, 
- and a message is sent.

```rust
use lifeline::lifeline_bus;
use lifeline::Message;
use lifeline::Bus;
use myapp::message::MainSend;
use myapp::message::MainRecv;
use tokio::sync::mpsc;

lifeline_bus!(pub struct MainBus);

impl Message<MainBus> for MainSend {
    type Channel = mpsc::Sender<Self>;
}

impl Message<MainBus> for MainRecv {
    type Channel = mpsc::Sender<Self>;
}

fn use_bus() -> anyhow::Result<()> {
    let bus = MainBus::default();

    let tx_main = bus.tx::<MainSend>()?;
    tx_main.send(MainSend {});

    Ok(())
}
```

The bus should be short-lived in the lifecycle of your application (i.e. drop it once your Main service has spawned).
This provides you with accurate 'channel disconnected' errors.

### A note about autocomplete
`rust-analyzer` does not currently support auto-import for structs defined in macros.  Lifeline really needs the
struct defined in the macro, as it injects magic fields which store the channels at runtime.

There is a workaround: define a `prelude.rs` file in your crate root that exports `pub use` for all your bus implementations.  
```
pub use lifeline::*;
pub use crate::bus::MainBus;
pub use crate::other::OtherBus;
...
```
Then in all your modules:
`use crate::prelude::*`

## The Carrier
Carriers provide a way to move messages between busses.  Carriers can translate, ignore, or collect information,
providing each bus with the messages that it needs.

Large applications have a tree of Busses.  This is good, it breaks your app into small chunks.
```
- Main
  | ConnectionListenerBus
  |  | ConnectionBus
  | DomainSpecificBus
  |  | ...
```
 They prevent an explosion of channel endpoints that are copied to all busses.

Carriers allow each bus can define messages that minimally represent the information it's services need to function.

Carriers centralize the communication between busses, making large applications easier to reason about.

### Carrier Example
Busses deeper in the tree should implement `FromCarrier` for their parents.

```rust
impl FromCarrier<MainBus> for ConnectionListenerBus {
    // if you only need one task, you can return Lifeline
    // if you need many tasks, you can return a struct like services do.

    type Lifeline = anyhow::Result<Lifeline>;
    fn carry_from(&self, from: &MainBus) -> Self::Lifeline {
        let mut rx_main = from.rx::<SomeMainMessage>()?;
        let mut tx_sub = self.tx::<SomeListenerMessage>()?;

        let lifeline = Self::try_task("from_main", async move {
            while let Some(msg) = rx_main.recv().await {
                match msg {
                    MainSend::HelloSubsurface => tx_sub.send(SubsurfaceSend::Hello {}).await?,
                    _ => {}
                }
            }

            Ok(())
        });

        Ok(lifeline)
    }
}
```

The carrier returns a lifeline.  When the lifeline is dropped, any tasks the carrier has spawned are immediately cancelled.
```rust
let main_bus = MainBus::default();
let connection_listener_bus = ConnectionListenerBus::default();
let _carrier = connection_listener_bus.carry_from(&main_bus)?;
// you can also use the IntoCarrier trait, which has a blanket implementation
let _carrier = main_bus.carry_into(&main_bus)?;
```

One note about ownership.  The `CarryFrom` / `CarryInto` traits do not consume the bus, as `std::convert::From` does.  But they
can take Senders/Receivers that are `!Clone`.  So they do consume resources, and are named `From`/`Into`.

## The Service
The Service takes channels from the Bus, and spawns a tree of tasks (which send & receive messages).  Returns one or more Lifeline values.  When the Lifeline is dropped, the task tree is immediately cancelled.

It's common for `Service::spawn` to return a Result.  Taking channel endpoints is a fallible operation.  This is because, depending on the channel type, the endpoint may not be clonable.  Lifeline clones endpoints when it can (mpsc::Sender, broadcast::*, and watch::Receiver).  Other endpoints are taken, removed, and future calls will return an Err.

The structure of `spawn` makes errors occur predictable and early.  If you get an `Err` on an `mpsc::Receiver`, change it's binding in the bus to `broadcast`.

```rust
use lifeline::{Service, Task};
pub struct ExampleService {
    _greet: Lifeline,
}

impl Service for ExampleService {
    type Bus = ExampleBus;
    type Lifeline = Lifeline;

    fn spawn(bus: &Self::Bus) -> Self::Lifeline {
        let mut rx = bus.rx::<ExampleSend>()?;
        let mut tx = bus.tx::<ExampleRecv>()?;
        let config = bus.resource::<ExampleConfig>()?;

        Self::task("greet", async move {
            // drive the channels!
        })
    }
}
```

## The Task
The Task executes a `Future`, and returns a Lifeline when spawned.  When the lifeline is dropped, the future is immediately cancelled.

`Task` is a trait that is implemented for all types - you can import it and use `Self::task` in any type.  In lifeline, it's 
most commonly used in Service implementations.

Tasks can be infallible:
```rust
Self::task("greet", async move {
    // do something
})
```

Or, if you have a fallible task:

```rust
Self::try_task("greet", async move {
    // do something
    let something = do_something()?;
    Ok(())
})
```

Note that the return type of the async closure is `anyhow::Result<T>`.  Requiring anyhow really simplifies the type inference on the task.
There is no need to infer an Err type, and the async block can return many types of `Err` with `?`.

# The Details
### Logging
Tasks (via [log](https://docs.rs/log/0.4.11/log/)) provide debug logs when the are started, ended, or cancelled.

If the task returns a value, it is also printed to the debug log using `Debug`.
```
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] START ExampleService/ok_task
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] END ExampleService/ok_task

2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] START ExampleService/valued_task
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] END ExampleService/valued_task: MyStruct {}
```

If the task is cancelled (because it's lifeline is dropped), that is also printed.
```
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] START ExampleService/cancelled_task
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] CANCEL ExampleService/cancelled_task
```

If the task is started using `Task::try_task`, the `Ok`/`Err` value will be printed with `Display`.

```
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] START ExampleService/ok_task
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] OK ExampleService/ok_task
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] END ExampleService/ok_task

2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] START ExampleService/err_task
2020-08-23 16:45:10,422 ERROR [lifeline::service] ERR: ExampleService/err_task: my error
2020-08-23 16:45:10,422 DEBUG [lifeline::spawn] END ExampleService/err_task
```

## The Resource
Resources can be stored on the bus.  This is very useful for configuration (e.g `MainConfig`), or connections (e.g. a `TcpStream`).

Resources implement the `Storage` trait, which is easy with the `impl_storage_clone!` and `impl_storage_take!` macros.

```rust
use lifeline::{lifeline_bus, impl_storage_clone};
lifeline_bus!(MainBus);
pub struct MainConfig {
    pub port: u16
}

impl_storage_clone!(MainConfig);

fn main() {
    let bus = MainBus::default()
    bus.store_resource::<MainConfig>(MainConfig { port: 12345 });
    // from here
}
```

Lifeline does not provide `Resource` implementations for Channel endpoints - use `bus.rx()` and `bus.tx()`.

## The Channel
Channel senders must implement the `Channel` trait to be usable in an `impl Message` binding.

In most cases, the `Channel` endpoints just implement `Storage`, which determines whether to 'take or clone' the endpoint on a `bus.rx()` or `bus.tx()` call.

Here is an example implem
```rust
use lifeline::Channel;
use crate::{impl_channel_clone, impl_channel_take};
use tokio::sync::{broadcast, mpsc, oneshot, watch};

impl<T: Send + 'static> Channel for mpsc::Sender<T> {
    type Tx = Self;
    type Rx = mpsc::Receiver<T>;

    fn channel(capacity: usize) -> (Self::Tx, Self::Rx) {
        mpsc::channel(capacity)
    }

    fn default_capacity() -> usize {
        16
    }
}

impl_channel_clone!(mpsc::Sender<T>);
impl_channel_take!(mpsc::Receiver<T>);
```

Broadcast senders should implement the trait with the `clone_rx` method overriden, to take from `Rx`, then subscribe to `Tx`.

# Testing
One of the goals of Lifeline is to provide interfaces that are very easy to test.  Lifeline runtimes are easy to construct in tests:

```rust
#[tokio::test]
async fn test() -> anyhow::Result<()> {
    // this is zero-cost.  channel construction is lazy.
    let bus = MainBus::default();
    let service = MainService::spawn(&bus)?;

    // the service took `bus.rx::<MainSend>()`
    //                + `bus.tx::<MainRecv>()`
    // let's communicate using channels we take.
    let tx = bus.tx::<MainSend>()?;
    let rx = bus.rx::<MainRecv>()?;

    // drop the bus, so that any 'channel closed' errors will occur during our test.
    // this would likely happen in practice during the long lifetime of the program
    drop(bus);

    tx.send(MainSend::MyMessage)?;

    // wait up to 200ms for the message to arrive
    // if we remove the 200 at the end, the default is 50ms
    lifeline::assert_completes!(async move {
        let response = rx.recv().await;
        assert_eq!(MainRecv::MyResponse, response);
    }, 200);

    Ok(())
}
```

## A note on assert_completes!
`assert_completes!` is really critical.  The problem with testing against channels is that one thing we need to test is
 'does a message ever arrive?'.

If we immediately `try_recv` for a message, there may be asynchronous tasks within the MainService (or the services that it spawns) that haven't caught up.  The only way to do this is with a timeout.

The upside is, in the success case (a message did arrive), we don't have to pay the timeout cost.  As soon as it arrives, the test passes and terminates.
