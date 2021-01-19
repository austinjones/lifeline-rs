use bus::ExampleBus;
use lifeline::{assert_completes, assert_times_out, prelude::*};
use lifeline::{Receiver, Sender};
use message::{DomainShutdown, MainRecv, MainShutdown};
use service::MainService;
use simple_logger::SimpleLogger;

/// This example shows how to propagate shutdown events, and synchronize shutdown with local tasks.
/// For documentation on basic concepts (bus/service/channels), see the 'hello' example.
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    SimpleLogger::new().init().expect("log init failed");

    let bus = ExampleBus::default();

    let _service = MainService::spawn(&bus)?;

    let mut tx = bus.tx::<MainRecv>()?;
    let mut tx_domain_shutdown = bus.tx::<DomainShutdown>()?;
    let mut rx_main_shutdown = bus.rx::<MainShutdown>()?;
    drop(bus);

    // let's send a few messages for the service to process.
    tx.send(MainRecv::Hello).await?;

    // and let's trigger a domain shutdown.  this will cause MainService to begin it's shutdown procedure
    tx_domain_shutdown.send(DomainShutdown {}).await?;

    // it shouldn't be ready yet though, because it waits for a goodbye message
    assert_times_out!(async { rx_main_shutdown.recv().await });

    // send the goodbye, now the service should have transmitted the shutdown message
    tx.send(MainRecv::Goodbye).await?;
    assert_completes!(async {
        let msg = rx_main_shutdown.recv().await;
        assert_eq!(Some(MainShutdown {}), msg);
    });

    println!("All done.");

    // in a real application, we would have here at the end:
    // rx_main_shutdown.recv().await

    // at the end of the scope, we drop the MainService value.
    // any tasks that the service (or the services it spawns) will immediately be cancelled
    // this makes it possible to locally reason about when spawned tasks will be terminated

    Ok(())
}

mod message {
    #[derive(Debug, Clone)]
    pub enum MainRecv {
        Hello,
        Goodbye,
    }

    /// This is the main shutdown event.  
    /// The main thread waits on this, and when received, it exits.n
    /// This causes all lifelines to be dropped and cancelled
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct MainShutdown;

    /// This is a domain-specific shutdown event.
    /// This can be triggered by a service which focuses on one app area.
    /// Main is responsible for interpreting this event, and acting on it.
    /// Main may need to shut down other services, or use Barriers to synchronize shutdown.
    #[derive(Debug, Clone)]
    pub struct DomainShutdown;

    /// This is a barrier message.  It's carried by a lifeline barrier channel
    /// Barrier channels behave a bit like oneshot channels.
    ///   but they produce a value when they are dropped.
    #[derive(Debug, Default, Clone)]
    pub struct MainEventBarrier;
}

mod bus {
    use crate::message::{DomainShutdown, MainEventBarrier, MainRecv, MainShutdown};
    use lifeline::prelude::*;
    use postage::barrier;
    use postage::mpsc;

    lifeline_bus!(pub struct ExampleBus);

    impl Message<ExampleBus> for MainRecv {
        type Channel = mpsc::Sender<Self>;
    }

    impl Message<ExampleBus> for DomainShutdown {
        type Channel = mpsc::Sender<Self>;
    }

    impl Message<ExampleBus> for MainShutdown {
        type Channel = mpsc::Sender<Self>;
    }

    impl Message<ExampleBus> for MainEventBarrier {
        type Channel = barrier::Sender;
    }
}

mod service {
    use super::bus::ExampleBus;
    use crate::message::{DomainShutdown, MainEventBarrier, MainRecv, MainShutdown};
    use lifeline::prelude::*;
    use postage::{sink::Sink, stream::Stream};

    pub struct MainService {
        _greet: Lifeline,
        _shutdown: Lifeline,
    }

    impl Service for MainService {
        type Bus = ExampleBus;
        type Lifeline = anyhow::Result<Self>;

        fn spawn(bus: &Self::Bus) -> Self::Lifeline {
            // Here we'll spawn a task which waits for a Goodbye message, then quits
            let _greet = {
                let mut rx = bus.rx::<MainRecv>()?;
                let mut tx_barrier = bus.tx::<MainEventBarrier>()?;
                Self::try_task("greet", async move {
                    while let Some(recv) = rx.recv().await {
                        if let MainRecv::Goodbye = recv {
                            break;
                        }
                    }

                    // send an event barrier message
                    // this would also occur automatically if the tx_barrier value was dropped
                    tx_barrier.send(()).await?;

                    Ok(())
                })
            };

            // And we'll spawn a shutdown task which synchronizes shutdown events
            let _shutdown = {
                let mut rx_domain_shutdown = bus.rx::<DomainShutdown>()?;
                let mut rx_barrier = bus.rx::<MainEventBarrier>()?;
                let mut tx_main_shutdown = bus.tx::<MainShutdown>()?;
                Self::try_task("shutdown", async move {
                    // if we receive a domain shutdown, begin the shutdown process
                    if let Some(_shutdown) = rx_domain_shutdown.recv().await {
                        // wait for the barrier to complete
                        rx_barrier.recv().await;
                        // forward the shutdown message, ignoring any tx error
                        // it's a good idea to do this in shutdown code,
                        // as receivers may have already gotten the message and dropped
                        tx_main_shutdown.send(MainShutdown {}).await.ok();
                    }

                    Ok(())
                })
            };

            Ok(Self { _greet, _shutdown })
        }
    }
}
