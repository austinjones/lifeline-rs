use bus::{MainBus, SubsurfaceBus};
use lifeline::FromCarrier;
use lifeline::{Bus, Service};
use message::{main::MainSend, subsurface::SubsurfaceRecv};
use service::HelloService;
use std::time::Duration;
use tokio::{sync::mpsc::error::TryRecvError, time::delay_for};

/// Spawn two busses, and create a task that drives messages between them
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    simple_logger::init().expect("log init failed");

    let main_bus = MainBus::default();
    let subsurface_bus = SubsurfaceBus::default();

    // the implementation is determined from the type of MainBus!
    // and it's type-safe - we can only call when it's implemented.
    let _carrier = subsurface_bus.carry_from(&main_bus)?;

    // if you try to spawn HelloService from main_bus, you'll get a compile error
    let _service = HelloService::spawn(&subsurface_bus)?;

    // let's pull MainSend off the main bus, to send some messages
    let mut tx = main_bus.tx::<MainSend>()?;

    // if you try to pull this from MainBus, you'll get a compile error
    let mut rx = subsurface_bus.rx::<SubsurfaceRecv>()?;

    // As soon as we are done pulling channels, we drop the busses.
    // The carrier will still run - it has grabbed all it's channels.
    drop(main_bus);
    drop(subsurface_bus);

    // let's send a few messages for the service to process.
    // in normal stack-based applications, these messages would compare to the arguments of the main function,

    tx.send(MainSend::Hello).await?;
    tx.send(MainSend::HelloSubsurface).await?;
    tx.send(MainSend::Goodbye).await?;

    let oh_hello = rx.recv().await;
    assert_eq!(Some(SubsurfaceRecv::OhHello), oh_hello);

    // the other messages we sent (Hello + Goodbye) weren't forwarded
    delay_for(Duration::from_millis(200)).await;
    let next_msg = rx.try_recv();
    assert_eq!(Err(TryRecvError::Empty), next_msg);

    println!("Subsurface says {:?}", oh_hello.unwrap());

    println!("All done.");

    Ok(())
}

/// These are the messages which our application uses to communicate.
/// The messages are carried over channels, using an async library (tokio, async_std, futures).
///
/// Send/Recv
mod message {
    // If a message is only used by one bus, I like to keep it in it's own module
    pub mod main {
        #[derive(Debug, Clone)]
        pub enum MainSend {
            Hello,
            HelloSubsurface,
            Goodbye,
        }
    }

    pub mod subsurface {
        #[derive(Debug, Clone)]
        pub enum SubsurfaceSend {
            Hello,
        }

        #[derive(Debug, Clone, Eq, PartialEq)]
        pub enum SubsurfaceRecv {
            OhHello,
        }
    }
}

/// This is the lifeline bus.
/// The bus carries channels (senders/receivers).
/// The bus knows how to construct these channels, and is lazy,
///   it constructs on demand.
/// The bus also carries resources, which are useful for cloneable config structs,
///   or structs required for initialization.
mod bus {
    use crate::message::{
        main::MainSend,
        subsurface::{SubsurfaceRecv, SubsurfaceSend},
    };
    use lifeline::Task;
    use lifeline::{lifeline_bus, Bus, FromCarrier, Lifeline, Message};
    use tokio::sync::mpsc;

    lifeline_bus!(pub MainBus);

    // This binds the message ExampleRecv to the bus.
    // We have to specify the channel sender!
    // The the channel sender must implement the Channel trait
    impl Message<MainBus> for MainSend {
        type Channel = mpsc::Sender<Self>;
    }

    lifeline_bus!(pub SubsurfaceBus);

    impl Message<SubsurfaceBus> for SubsurfaceSend {
        type Channel = mpsc::Sender<Self>;
    }

    impl Message<SubsurfaceBus> for SubsurfaceRecv {
        type Channel = mpsc::Sender<Self>;
    }

    impl FromCarrier<MainBus> for SubsurfaceBus {
        // if you only need one task, you can return Lifeline
        // if you need many tasks, you can return a struct like services do.

        type Lifeline = anyhow::Result<Lifeline>;
        fn carry_from(&self, from: &MainBus) -> Self::Lifeline {
            let mut rx_main = from.rx::<MainSend>()?;
            let mut tx_sub = self.tx::<SubsurfaceSend>()?;

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
}

mod service {
    use super::bus::SubsurfaceBus;
    use crate::message::subsurface::SubsurfaceRecv;
    use crate::message::subsurface::SubsurfaceSend;
    use lifeline::{Bus, Lifeline, Service, Task};

    pub struct HelloService {
        _greet: Lifeline,
    }

    impl Service for HelloService {
        type Bus = SubsurfaceBus;
        type Lifeline = anyhow::Result<Self>;

        fn spawn(bus: &Self::Bus) -> Self::Lifeline {
            // The generic args here are required, by design.
            // Type inference would be nice, but if you type the message name here,
            //   you can GREP THE NAME!  Just search an event name and you'll see:
            // - which bus(es) the event is carried on
            // - which services rx the event
            // - which services tx the event

            // also, rx before tx!  somewhat like fn service(rx) -> tx {}
            let mut rx = bus.rx::<SubsurfaceSend>()?;
            let mut tx = bus.tx::<SubsurfaceRecv>()?;

            let _greet = Self::try_task("greet", async move {
                while let Some(recv) = rx.recv().await {
                    match recv {
                        SubsurfaceSend::Hello => {
                            tx.send(SubsurfaceRecv::OhHello).await?;
                        }
                    }
                }

                Ok(())
            });

            Ok(Self { _greet })
        }
    }
}
