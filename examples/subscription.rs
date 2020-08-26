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

mod message {
    // We derive a Hash + Eq key that can be used on the bus
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub struct ExampleId(pub u64);
}

mod bus {
    use crate::message::ExampleId;
    use lifeline::{lifeline_bus, subscription, Message};

    lifeline_bus!(pub struct SubscriptionBus);

    // We bind a message of Subscription<ExampleId>, as that is the message type we send.
    // The receiver is aware of the type and is able to answer questions about subscription status.
    impl Message<SubscriptionBus> for subscription::Subscription<ExampleId> {
        type Channel = subscription::Sender<ExampleId>;
    }
}
