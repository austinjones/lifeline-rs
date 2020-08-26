// This example shows how to make a custom Sender/Receiver type compatible with the lifeline bus

use bus::ChannelBus;
use lifeline::Bus;
use lifeline::{Receiver, Sender};
use message::ExampleMessage;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let bus = ChannelBus::default();
    let mut tx = bus.tx::<ExampleMessage>()?;
    let rx = bus.rx::<ExampleMessage>()?;

    let sent = tx.send(ExampleMessage {}).await;
    assert_eq!(Ok(()), sent);

    associated(rx).await;

    Ok(())
}

/// The Sender & Receiver trait allows lifeline users to do this.  it also works with &mut impl Receiver<...>
/// It's really nice for switching between broadcast/mpsc channels on the bus.
/// Their service implementatons can just work, even with major changes to how channels are actually bound.
async fn associated(mut rx: impl Receiver<ExampleMessage>) {
    let rx = rx.recv().await;
    assert_eq!(None, rx);
}

mod sender {
    use crate::receiver::ExampleReceiver;
    use async_trait::async_trait;
    use lifeline::error::SendError;
    use lifeline::{impl_channel_clone, Channel, Sender};
    use std::{fmt::Debug, marker::PhantomData};

    /// Define a dummy Sender type that implements clone

    pub struct ExampleSender<T> {
        _t: PhantomData<T>,
    }

    impl<T> Clone for ExampleSender<T> {
        fn clone(&self) -> Self {
            Self { _t: PhantomData }
        }
    }

    impl<T> ExampleSender<T> {
        pub fn new(_capacity: usize) -> Self {
            Self { _t: PhantomData }
        }

        /// Define a dummy send method that either 'sends' with Ok, or returns the value to the caller.
        /// This would be the real send implementation, that communicates with the Receiver.
        pub fn send(_value: T) -> Result<(), T> {
            Ok(())
        }
    }

    /// Implement the 'Sender' trait for ExampleSender.
    /// If users 'use' the Sender trait, the Sender API will shadow our custom method.
    /// This provides a consistent API for users, who can switch between watch, mpsc, and broadcast channels.
    /// They also can write `impl Sender<T>` for associated methods on Service impls.
    #[async_trait]
    impl<T> Sender<T> for ExampleSender<T>
    where
        T: Debug + Send,
    {
        async fn send(&mut self, value: T) -> Result<(), SendError<T>> {
            ExampleSender::send(value).map_err(|value| SendError::Return(value))
        }
    }

    // Implement a 'clone' operation for `bus.rx::<T>()`
    // This channel can be taken any number of times for the lifetime of the bus.
    // Note that this *doesn't* mean the channel endpoint can be moved to another bus.
    // That requires a Carrier.
    impl_channel_clone!(ExampleSender<T>);

    /// This is the trait that the bus uses to construct the channel.
    /// It can be generic over T, but is implement for the channel Sender type.
    impl<T> Channel for ExampleSender<T>
    where
        T: Send + 'static,
    {
        // this controls the return type of the `bus.tx()` and `bus.rx()` call.
        type Tx = Self;
        type Rx = ExampleReceiver<T>;

        // this is what the bus calls when it want's to 'link' a channel
        //   (e.g. construct an endpoint pair, and storing in it's 'slot' for the channel)
        // channel endpoints can only be linked once in the lifetime of the bus.
        fn channel(capacity: usize) -> (Self::Tx, Self::Rx) {
            // construct and return the linked pair
            let tx = ExampleSender::new(capacity);
            let rx = ExampleReceiver::new();

            (tx, rx)
        }

        // this controls the default channel capacity.
        // users can override this with `bus.capacity::<Msg>(42)`
        fn default_capacity() -> usize {
            100
        }
    }
}

mod receiver {
    use async_trait::async_trait;
    use lifeline::{impl_channel_take, Receiver};
    use std::{fmt::Debug, marker::PhantomData};

    /// Define a dummy Sender type that implements clone
    #[derive(Clone)]
    pub struct ExampleReceiver<T> {
        _t: PhantomData<T>,
    }

    pub struct CustomError {}

    impl<T> ExampleReceiver<T> {
        pub fn new() -> Self {
            Self { _t: PhantomData }
        }

        /// Define a dummy recv method that receives
        /// Note that this may return an error, but we will be forced to discard it in the Receiver implementation
        pub async fn recv() -> Result<T, CustomError> {
            Err(CustomError {})
        }
    }

    /// Implement the 'Receiver' trait for ExampleReceiver.
    /// Note the return type of Option<T> - this is because a disconnected sender is not an error for a service
    /// It is simply a termination condition.
    /// Some channel types (broadcast) have additional error conditions.
    /// Those should be logged at warn level, or ignored.
    #[async_trait]
    impl<T> Receiver<T> for ExampleReceiver<T>
    where
        T: Debug + Send,
    {
        async fn recv(&mut self) -> Option<T> {
            ExampleReceiver::recv().await.ok()
        }
    }

    // Implement a 'take' operation for `bus.rx::<T>()`
    // Once taken, future calls will return an Err.
    impl_channel_take!(ExampleReceiver<T>);
}

mod message {
    #[derive(Clone, Debug, PartialEq)]
    pub struct ExampleMessage {}
}

mod bus {
    use crate::{message::ExampleMessage, sender::ExampleSender};
    use lifeline::prelude::*;

    lifeline_bus!(pub struct ChannelBus);

    // we link the message type, and the sender type here.
    // the bus requires the implementation of Channel and Storage.
    // the Sender/Receiver traits are optional,
    //   but they significantly improve ergonomics for the user
    impl Message<ChannelBus> for ExampleMessage {
        type Channel = ExampleSender<Self>;
    }
}
