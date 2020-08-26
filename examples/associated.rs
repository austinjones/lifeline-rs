use bus::ExampleBus;
use lifeline::prelude::*;
use message::*;
use service::ExampleService;

/// If a service spawns many tasks, it helps to break the run functions up.  
/// You can do this by defining associated functions on the service type,
///   and calliing them with Self::run_something(rx, tx)  
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let bus = ExampleBus::default();
    let _service = ExampleService::spawn(&bus)?;

    let mut rx = bus.rx::<ExampleSend>()?;
    let mut tx = bus.tx::<ExampleRecv>()?;

    tx.send(ExampleRecv::Hello).await?;

    let oh_hello = rx.recv().await;
    assert_eq!(Some(ExampleSend::OhHello), oh_hello);
    println!("Service says {:?}", oh_hello.unwrap());

    Ok(())
}

mod bus {
    use crate::message::*;
    use lifeline::prelude::*;
    use tokio::sync::broadcast;

    lifeline_bus!(pub struct ExampleBus);

    impl Message<ExampleBus> for ExampleRecv {
        type Channel = broadcast::Sender<Self>;
    }

    impl Message<ExampleBus> for ExampleSend {
        type Channel = broadcast::Sender<Self>;
    }
}

mod message {
    #[derive(Debug, Clone)]
    pub enum ExampleRecv {
        Hello,
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub enum ExampleSend {
        OhHello,
    }
}

mod service {
    use crate::bus::ExampleBus;
    use crate::message::*;
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
            let rx = bus.rx::<ExampleRecv>()?;
            let tx = bus.tx::<ExampleSend>()?;

            let _greet = Self::try_task("greet", Self::run_greet(rx, tx));

            Ok(Self { _greet })
        }
    }

    impl ExampleService {
        async fn run_greet(
            mut rx: impl Receiver<ExampleRecv>,
            mut tx: impl Sender<ExampleSend>,
        ) -> anyhow::Result<()> {
            while let Some(recv) = rx.recv().await {
                match recv {
                    ExampleRecv::Hello => {
                        tx.send(ExampleSend::OhHello).await?;
                    }
                }
            }

            Ok(())
        }
    }
}
