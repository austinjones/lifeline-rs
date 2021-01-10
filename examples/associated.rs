use bus::ExampleBus;
use lifeline::prelude::*;
use message::*;
use postage::{Sink, Stream};
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
    use postage::broadcast;

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
    use lifeline::{Receiver, Sender};

    pub struct ExampleService {
        _greet: Lifeline,
    }

    impl Service for ExampleService {
        type Bus = ExampleBus;
        type Lifeline = anyhow::Result<Self>;

        fn spawn(bus: &Self::Bus) -> Self::Lifeline {
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
