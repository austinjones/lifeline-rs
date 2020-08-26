use bus::SubscriptionBus;
use lifeline::{prelude::*, subscription::Subscription};
use message::ExampleId;
use time::Duration;
use tokio::time;

/// The subscription service maintains a list of subscribed entries.
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    simple_logger::init().expect("log init failed");

    let bus = SubscriptionBus::default();

    let rx = bus.rx::<Subscription<ExampleId>>()?.into_inner();
    let mut tx = bus.tx::<Subscription<ExampleId>>()?.into_inner();

    // updates are asynchronous.
    // they are processed using lifeline that is stored in the sender
    tx.send(Subscription::Subscribe(ExampleId(1))).await?;
    time::delay_for(Duration::from_millis(100)).await;

    // the receiver can check whether an id is contained in the subscription
    assert!(rx.contains(&ExampleId(1)));
    assert_eq!(false, rx.contains(&ExampleId(2)));

    // the channel also generates unique identifiers for each subscription,
    // so an unsubscribe/subscribe can be handled uniquely.
    assert_eq!(Some(0), rx.get_identifier(&ExampleId(1)));
    assert_eq!(None, rx.get_identifier(&ExampleId(2)));

    println!("All done.");

    Ok(())
}

/// These are the messages which our application uses to communicate.
/// The messages are carried over channels, using an async library (tokio, async_std, futures).
///
/// Send/Recv
mod message {
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub struct ExampleId(pub u64);
}

/// This is the lifeline bus.
/// The bus carries channels (senders/receivers).
/// The bus knows how to construct these channels, and is lazy,
///   it constructs on demand.
/// The bus also carries resources, which are useful for cloneable config structs,
///   or structs required for initialization.
mod bus {
    use crate::message::ExampleId;
    use lifeline::{lifeline_bus, subscription, Message};

    // This is a macro that generates an ExampleBus struct,
    //   and implements DynBus for it.
    // DynBus stores the channels in Box<dyn Any> slots,
    //  and deals with all the dyn trait magic for us.
    lifeline_bus!(pub struct SubscriptionBus);

    // This binds the message ExampleRecv to the bus.
    // We have to specify the channel sender!
    // The the channel sender must implement the Channel trait
    impl Message<SubscriptionBus> for subscription::Subscription<ExampleId> {
        type Channel = subscription::Sender<ExampleId>;
    }
}
