use bus::StateBus;
use lifeline::prelude::*;
use message::MainRecv;
use service::{MainService, StateService};
use simple_logger::SimpleLogger;
use state::{LocationState, SkyState, WeatherState};
use std::time::Duration;
use tokio::time::sleep;

/// This example shows how to maintain state in a service, and broadcast it using channels.
/// For documentation on basic concepts (bus/service/channels), see the 'hello' example.
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    SimpleLogger::new().init().expect("log init failed");

    let bus = StateBus::default();

    let _service = MainService::spawn(&bus)?;
    let _state = StateService::spawn(&bus)?;

    let mut tx = bus.tx::<MainRecv>()?;
    let mut rx = bus.rx::<SkyState>()?;

    // The bus *stores* channel endpoints.
    // As soon as your bus has been used to spawn your service,
    //  and take your channels, drop it!
    // Then your tasks will get correct 'disconnected' Nones/Errs.
    drop(bus);

    // let's send a few messages for the service to process.
    // in normal stack-based applications, these messages would compare to the arguments of the main function,
    tx.send(MainRecv::Travel(LocationState::Boston)).await?;

    // state updates are asynchronous.  they may not be propagated immediately
    sleep(Duration::from_millis(20)).await;

    let state = rx.recv().await;
    let expected = SkyState {
        weather: WeatherState::Snowing,
        location: LocationState::Boston,
    };

    // it's snowing in boston!
    assert_eq!(Some(expected), state);

    //
    // let's travel to san diego!
    //
    tx.send(MainRecv::Travel(LocationState::SanDiego)).await?;

    // state updates are asynchronous.  they may not be propagated immediately
    sleep(Duration::from_millis(20)).await;

    let state = rx.recv().await;
    let expected = SkyState {
        weather: WeatherState::Sunny72Degrees,
        location: LocationState::SanDiego,
    };

    // it's snowing in boston!
    assert_eq!(Some(expected), state);

    println!("All done.");

    Ok(())
}

/// These are the messages which our application uses to communicate.
/// The messages are carried over channels, using an async library (tokio, async_std, futures).
///
/// Send/Recv
mod message {
    // You might be tempted to write a struct here for MainRecv.
    // You can do that, but I like to write enums for service send/recvs.
    // It's much easier to add message types!

    use crate::state::LocationState;

    // If the message is not tied to the service recv (e.g. WeatherEvent),
    //  then it's nice to write a struct.
    // Then multiple services can subscribe via a broadcast channel, and consume the event.
    #[derive(Debug, Clone)]
    pub enum MainRecv {
        Travel(LocationState),
    }

    // This is a one-off event.
    // It's carried on the bus, and isn't 'owned' by a service.
    // If the channel is mpsc, there can only be one receiver.
    // If the channel is broadcast, many services can send/receive the events.
    #[derive(Debug, Clone)]
    pub struct TravelEvent(pub LocationState);
}

// I like to keep state in a separate module.
// State is very different from channels.
// It is persistent, and it changes.
// Messages are just transmitted and then immediately disposed.
mod state {
    // This is a State struct.
    // It is mainained by a service, cloned, and commmunicated via channels.
    // Use pub fields if you need to communicate multiple values, or just a top-level enum.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct SkyState {
        pub weather: WeatherState,
        pub location: LocationState,
    }

    // Name your state structs with the State postfix!
    // Even though state is maintained in a service, it comes from 'the world'.
    // The service that maintains the state 'receives' it (though it may calculate it).
    // The service that uses the state 'recieves' it.
    // So the Send/Recv postfixes don't make sense.
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub enum WeatherState {
        None,
        Snowing,
        Sunny72Degrees,
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub enum LocationState {
        None,
        Boston,
        SanDiego,
    }

    // States communicated over channels must implement Default!
    // This is because the Bus needs to initialize the channels without any arguments.
    impl Default for SkyState {
        fn default() -> Self {
            Self {
                weather: WeatherState::None,
                location: LocationState::None,
            }
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
    use crate::message::{MainRecv, TravelEvent};
    use crate::state::SkyState;
    use lifeline::prelude::*;
    use tokio::sync::{broadcast, mpsc, watch};

    lifeline_bus!(pub struct StateBus);

    // we bind a watch sender here.
    // watch senders store the latest value,
    // and allow the receiver to either borrow, or clone.
    impl Message<StateBus> for SkyState {
        type Channel = watch::Sender<Self>;
    }

    // We bind a broadcast sender for events.
    // In this example, it isn't necessary, but it's useful in big apps.
    // If you want to downgrade a broadcast to mpsc, do it here, run your app,
    //  and see if you get a TakeChannelError on service spawn.
    impl Message<StateBus> for TravelEvent {
        type Channel = broadcast::Sender<Self>;
    }

    impl Message<StateBus> for MainRecv {
        type Channel = mpsc::Sender<Self>;
    }
}

/// This is the service.
/// The service is a spawnable task that launches from the bus.
/// Service spawn is **synchronous** - the spawn should not send/receive messages, and it should be branchless.
/// This makes errors very predictable.  If you take an MPSC receiver twice, you immediately get the error on startup.
mod service {
    use super::bus::StateBus;
    use crate::{
        message::{MainRecv, TravelEvent},
        state::{SkyState, WeatherState},
    };
    use lifeline::prelude::*;

    pub struct MainService {
        _greet: Lifeline,
    }

    impl Service for MainService {
        type Bus = StateBus;
        type Lifeline = anyhow::Result<Self>;

        fn spawn(bus: &Self::Bus) -> Self::Lifeline {
            let mut rx = bus.rx::<MainRecv>()?;
            let mut tx = bus.tx::<TravelEvent>()?;

            let _greet = Self::try_task("greet", async move {
                while let Some(recv) = rx.recv().await {
                    match recv {
                        MainRecv::Travel(location) => {
                            tx.send(TravelEvent(location)).await?;
                        }
                    }
                }

                Ok(())
            });

            Ok(Self { _greet })
        }
    }

    // The state service keeps track of the state
    pub struct StateService {
        _travel: Lifeline,
    }

    impl Service for StateService {
        type Bus = StateBus;
        type Lifeline = anyhow::Result<Self>;

        fn spawn(bus: &Self::Bus) -> Self::Lifeline {
            let mut rx = bus.rx::<TravelEvent>()?;

            // if you need to get to the original channel, you can use into_inner()
            // this will make your code more fragile when you change the types in the bus, though!
            let tx = bus.tx::<SkyState>()?.into_inner();

            let _travel = Self::try_task("travel", async move {
                // default is nice!  we can initialize to the same value as the tx stores!
                let mut state = SkyState::default();

                // there is a small bug here w/ broadcast lagged error.
                // ignoring it for simplicity :D
                while let Some(update) = rx.recv().await {
                    state.location = update.0;
                    match state.location {
                        crate::state::LocationState::None => state.weather = WeatherState::None,
                        crate::state::LocationState::Boston => {
                            state.weather = WeatherState::Snowing
                        }
                        crate::state::LocationState::SanDiego => {
                            state.weather = WeatherState::Sunny72Degrees
                        }
                    }

                    // rx/tx errors should stop the task!
                    // this is actually useful - disconnected channels propagate shutdowns.
                    // in this case, if all the receivers have disconnected,
                    //   we can stop calculating the state.
                    tx.send(state.clone())?;
                }

                Ok(())
            });

            Ok(Self { _travel })
        }
    }
}
