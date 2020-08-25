use bus::ExampleBus;
use lifeline::prelude::*;
use message::{ExampleRecv, ExampleSend};
use service::ExampleService;

/// Spawn a simple bus, and a service
/// The service execution is tied to the 'lifeline' it returns
/// If `service` is dropped, all it's tasks are cancelled.
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    simple_logger::init().expect("log init failed");

    // Bus construction is immediate, parameterless, and infallible.
    // All busses implement Default.
    let bus = ExampleBus::default();

    // The value returned here is a *lifeline*.
    // The tasks spawned by the service to drive channels *immediately* stop when the service is dropped.
    // This means that when you construct a service, you control how long it runs for.
    // You can get a sense of what tasks a service runs by looking at the struct definition.
    let _service = ExampleService::spawn(&bus)?;

    // there is an important naming convention here
    // tx - for Sender channels
    // rx - for Recevier channels
    // ExampleSend - a message which is sent to the service (and the service receives)
    // ExampleRecv - a message which is sent to the world (and the service sends)

    // this side of the channel is 'covariant'.
    //   we tx a 'send' msg, and rx a 'recv' message.
    // if we were in the service,
    //   we would tx a 'recv' message, and 'rx' a send message
    // this naming convention helps a lot when reading code

    // taking receivers out of the bus is fallible.  behavior depends on the channel type
    // lifeline is designed to make failures predictable, and early (near bus construction).
    // it also keeps track of as much context as possible using anyhow.
    // when rx/tx are called, channels are either cloned (remain in the bus for other takers), or taken.
    // in general:
    //   mpsc:       clone Sender / take  Receiver
    //   broadcast:  clone Sender / clone Receiver
    //   oneshot:    take Sender  / take  Receiver
    //   watch:      take Sender  / clone Receiver
    let mut rx = bus.rx::<ExampleSend>()?;
    let mut tx = bus.tx::<ExampleRecv>()?;

    // The bus *stores* channel endpoints.
    // As soon as your bus has been used to spawn your service,
    //  and take your channels, drop it!
    // Then your tasks will get correct 'disconnected' Nones/Errs.
    drop(bus);

    // let's send a few messages for the service to process.
    // in normal stack-based applications, these messages would compare to the arguments of the main function,
    tx.send(ExampleRecv::Hello).await?;
    tx.send(ExampleRecv::Goodbye).await?;

    let oh_hello = rx.recv().await;
    assert_eq!(Some(ExampleSend::OhHello), oh_hello);
    println!("Service says {:?}", oh_hello.unwrap());

    let aww_ok = rx.recv().await;
    assert_eq!(Some(ExampleSend::AwwOk), aww_ok);
    println!("Service says {:?}", aww_ok.unwrap());

    println!("All done.");

    Ok(())
}

/// These are the messages which our application uses to communicate.
/// The messages are carried over channels, using an async library (tokio, async_std, futures).
///
/// Send/Recv
mod message {
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub enum ExampleSend {
        OhHello,
        AwwOk,
    }

    #[derive(Debug, Clone)]
    pub enum ExampleRecv {
        Hello,
        Goodbye,
    }
}

/// This is the lifeline bus.
/// The bus carries channels (senders/receivers).
/// The bus knows how to construct these channels, and is lazy,
///   it constructs on demand.
/// The bus also carries resources, which are useful for cloneable config structs,
///   or structs required for initialization.
mod bus {
    use crate::message::{ExampleRecv, ExampleSend};
    use lifeline::{lifeline_bus, Message};
    use tokio::sync::mpsc;

    // This is a macro that generates an ExampleBus struct,
    //   and implements DynBus for it.
    // DynBus stores the channels in Box<dyn Any> slots,
    //  and deals with all the dyn trait magic for us.
    lifeline_bus!(pub struct ExampleBus);

    // This binds the message ExampleRecv to the bus.
    // We have to specify the channel sender!
    // The the channel sender must implement the Channel trait
    impl Message<ExampleBus> for ExampleSend {
        type Channel = mpsc::Sender<Self>;
    }

    impl Message<ExampleBus> for ExampleRecv {
        type Channel = mpsc::Sender<Self>;
    }
}

/// This is the service.
/// The service is a spawnable task that launches from the bus.
/// Service spawn is **synchronous** - the spawn should not send/receive messages, and it should be branchless.
/// This makes errors very predictable.  If you take an MPSC receiver twice, you immediately get the error on startup.
mod service {
    use super::bus::ExampleBus;
    use crate::message::{ExampleRecv, ExampleSend};
    use lifeline::prelude::*;

    pub struct ExampleService {
        _greet: Lifeline,
    }

    impl Service for ExampleService {
        type Bus = ExampleBus;
        type Lifeline = anyhow::Result<Self>;

        fn spawn(bus: &Self::Bus) -> Self::Lifeline {
            // The generic args here are required, by design.
            // Type inference would be nice, but if you type the message name here,
            //   you can GREP THE NAME!  Just search an event name and you'll see:
            // - which bus(es) the event is carried on
            // - which services rx the event
            // - which services tx the event

            // also, rx before tx!  somewhat like fn service(rx) -> tx {}
            let mut rx = bus.rx::<ExampleRecv>()?;
            let mut tx = bus.tx::<ExampleSend>()?;

            let _greet = Self::try_task("greet", async move {
                while let Some(recv) = rx.recv().await {
                    match recv {
                        ExampleRecv::Hello => {
                            tx.send(ExampleSend::OhHello).await?;
                        }
                        ExampleRecv::Goodbye => {
                            tx.send(ExampleSend::AwwOk).await?;
                        }
                    }
                }

                Ok(())
            });

            Ok(Self { _greet })
        }
    }
}
