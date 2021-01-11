//! A copy of the hello.rs example, using the async_std runtime
//!
//! Can be run using: `cargo run --example async-std --no-default-features --features="dyn-bus async-std-executor async-std-channels"`

use bus::ExampleBus;
use lifeline::prelude::*;
use message::{ExampleRecv, ExampleSend};
use service::ExampleService;
use std::time::Duration;

#[async_std::main]
pub async fn main() -> anyhow::Result<()> {
    let bus = ExampleBus::default();
    let _service = ExampleService::spawn(&bus)?;

    let mut rx = bus.rx::<ExampleSend>()?;
    let mut tx = bus.tx::<ExampleRecv>()?;

    drop(bus);

    tx.send(ExampleRecv::Hello).await?;
    tx.send(ExampleRecv::Goodbye).await?;

    let oh_hello = rx.recv().await;
    assert_eq!(Some(ExampleSend::OhHello), oh_hello);
    println!("Service says {:?}", oh_hello.unwrap());

    let aww_ok = rx.recv().await;
    assert_eq!(Some(ExampleSend::AwwOk), aww_ok);
    println!("Service says {:?}", aww_ok.unwrap());

    println!("All done.");

    // For some reason, async_std panics due to printlns on shutdown
    async_std::task::sleep(Duration::from_millis(100)).await;

    Ok(())
}

/// These are the messages which our application uses to communicate.
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
mod bus {
    use crate::message::{ExampleRecv, ExampleSend};
    use lifeline::{lifeline_bus, Message};

    // This is a macro that generates an ExampleBus struct,
    //   and implements DynBus for it.
    lifeline_bus!(pub struct ExampleBus);

    impl Message<ExampleBus> for ExampleSend {
        type Channel = async_std::channel::Sender<Self>;
    }

    impl Message<ExampleBus> for ExampleRecv {
        type Channel = async_std::channel::Sender<Self>;
    }
}

/// This is the service.
/// The service is a spawnable task that launches from the bus.
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
