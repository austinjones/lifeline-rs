use bus::{MainBus, SubsurfaceBus};
use lifeline::prelude::*;
use message::{main::MainSend, subsurface::SubsurfaceSend};
use service::HelloService;
use std::time::Duration;
use tokio::{sync::mpsc::error::TryRecvError, time::delay_for};

/// This examples shows how to communicate between Bus instances using the CarryFrom trait
/// When your application gets large, eventually you need to spawn new tasks as runtime (when a connection arrives)
/// At that point, you should create a new Bus type, and a Bus instance for each connection.
/// Your new bus probably needs to communicate with your main application bus, and you can use a Carrier to do this.
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
    let mut rx = subsurface_bus.rx::<SubsurfaceSend>()?;

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
    assert_eq!(Some(SubsurfaceSend::OhHello), oh_hello);
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
        #[derive(Debug, Clone, Eq, PartialEq)]
        pub enum SubsurfaceSend {
            OhHello,
        }

        #[derive(Debug, Clone)]
        pub enum SubsurfaceRecv {
            Hello,
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
    use lifeline::prelude::*;
    use tokio::sync::mpsc;

    lifeline_bus!(pub struct MainBus);

    // This binds the message MainSend to the bus.
    // We have to specify the channel sender!
    // The the channel sender must implement the Channel trait
    impl Message<MainBus> for MainSend {
        type Channel = mpsc::Sender<Self>;
    }

    lifeline_bus!(pub struct SubsurfaceBus);

    impl Message<SubsurfaceBus> for SubsurfaceRecv {
        type Channel = mpsc::Sender<Self>;
    }

    impl Message<SubsurfaceBus> for SubsurfaceSend {
        type Channel = mpsc::Sender<Self>;
    }

    impl CarryFrom<MainBus> for SubsurfaceBus {
        // if you only need one task, you can return Lifeline
        // if you need many tasks, you can return a struct like services do.

        type Lifeline = anyhow::Result<Lifeline>;
        fn carry_from(&self, from: &MainBus) -> Self::Lifeline {
            let mut rx_main = from.rx::<MainSend>()?;
            let mut tx_sub = self.tx::<SubsurfaceRecv>()?;

            let lifeline = Self::try_task("from_main", async move {
                while let Some(msg) = rx_main.recv().await {
                    match msg {
                        MainSend::HelloSubsurface => tx_sub.send(SubsurfaceRecv::Hello {}).await?,
                        _ => {}
                    }
                }

                Ok(())
            });

            Ok(lifeline)
        }
    }
}

/// This is the service.
/// The service is a spawnable task that launches from the bus.
/// Service spawn is **synchronous** - the spawn should not send/receive messages, and it should be branchless.
/// This makes errors very predictable.  If you take an MPSC receiver twice, you immediately get the error on startup.
mod service {
    use super::bus::SubsurfaceBus;
    use crate::message::subsurface::SubsurfaceRecv;
    use crate::message::subsurface::SubsurfaceSend;
    use lifeline::prelude::*;

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
            let mut rx = bus.rx::<SubsurfaceRecv>()?;
            let mut tx = bus.tx::<SubsurfaceSend>()?;

            let _greet = Self::try_task("greet", async move {
                while let Some(recv) = rx.recv().await {
                    match recv {
                        SubsurfaceRecv::Hello => {
                            tx.send(SubsurfaceSend::OhHello).await?;
                        }
                    }
                }

                Ok(())
            });

            Ok(Self { _greet })
        }
    }
}
