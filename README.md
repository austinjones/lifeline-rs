# Lifeline
Lifeline is a dependency injection library for message-based applications.

Lifeline provides:
 - The Bus, which constructs & distributes Senders, Receivers, and Resources.
 - The Service, which takes channels from the bus, and drives messages along the channels.
 - The Task, an async future which returns a lifeline when spawned.  When the lifeline is dropped, the future is immedately cancelled.
 - The Resource, a struct which can be stored in the bus, and taken (or cloned) when services spawn.


## The Bus
The bus carries channels and resources.  When services spawn, they receive a reference to the bus.

Channels can be taken from the bus.  If the channel endpoint implements is clonable, it will remain available for other service.  
But if the channel is `!Clone`, future calls will get an `Err`.  The Rx/Tx type parameters are type-safe, and will produce a compile error
if you attempt to take a channel for an message type which the bus does not carry.

```rust
lifeline_bus!(pub MainBus);

let rx = bus.rx::<MessageType>()?;
```



```rust
use lifeline::lifeline_bus;
use lifeline::Message;
use lifeline::Bus;
use myapp::message::MainSend;
use myapp::message::MainRecv;
use tokio::sync::mpsc;

lifeline_bus!(pub MainBus);

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

## The Service
The service takes channel endpoints from the bus, and spawns tasks.  

Taking channel endpoints and resources is fallible.  This is because, depending on the channel type, the endpoint may not be clonable.
Lifeline clones endpoints when it can (mpsc::Sender, broadcast::*, and watch::Receiver).  Other endpoints are taken, removed, and future calls will return an Err.

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
The Task executes an `Future`, but returns a Lifeline when spawned.  

`Task` is a trait that is implemented for all types - you can import it and use `Self::task` in any type, but in Lifeline it's used within `Service` implementations.

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