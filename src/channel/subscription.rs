//! A subscription utility channel, which can maintain a subscription state, and generate unique identifiers for each new subscription.
use super::Channel;
use crate::{Bus, Service};
pub use channel::{Receiver, Sender};
pub use messages::{Subscription, SubscriptionState};
use std::{fmt::Debug, hash::Hash};

impl<T> Channel for channel::Sender<T>
where
    T: Hash + Eq + Clone + Debug + Send + Sync + 'static,
{
    type Tx = channel::Sender<T>;
    type Rx = channel::Receiver<T>;

    fn channel(capacity: usize) -> (Self::Tx, Self::Rx) {
        let bus = bus::SubscriptionBus::default();
        bus.capacity::<messages::Subscription<T>>(capacity)
            .expect("capacity Subscription<T>");

        let service = service::UpdateService::spawn(&bus).expect("failed to spawn service");

        let tx = bus
            .tx::<messages::Subscription<T>>()
            .expect("tx Subscription<T>");

        let rx = bus
            .rx::<messages::SubscriptionState<T>>()
            .expect("rx SubscriptionState<T>");

        let sender = channel::Sender::new(tx.into_inner(), service);

        let receiver = channel::Receiver::new(rx.into_inner());
        (sender, receiver)
    }

    fn default_capacity() -> usize {
        32
    }
}

mod bus {
    use crate::{lifeline_bus, Message};
    use std::{fmt::Debug, hash::Hash};
    use tokio::sync::{mpsc, watch};

    lifeline_bus!(
        pub struct SubscriptionBus<T>
    );

    impl<T> Message<SubscriptionBus<T>> for super::messages::Subscription<T>
    where
        T: Debug + Send + Sync + 'static,
    {
        type Channel = mpsc::Sender<Self>;
    }

    impl<T> Message<SubscriptionBus<T>> for super::messages::SubscriptionState<T>
    where
        T: Clone + Hash + PartialEq + Debug + Send + Sync + 'static,
    {
        type Channel = watch::Sender<Self>;
    }
}

mod channel {
    use super::messages::{Subscription, SubscriptionState};
    use crate::error::SendError as LifelineSendError;
    use crate::{impl_channel_clone, Lifeline};
    use async_trait::async_trait;
    use std::{fmt::Debug, hash::Hash, sync::Arc};
    use tokio::sync::{mpsc, watch};

    /// A sender which takes `Subscription::Subscribe(id)` and `Subscription::Unsubscribe(id)` messages
    #[derive(Debug)]
    pub struct Sender<T> {
        tx: mpsc::Sender<Subscription<T>>,
        // TODO: store in sender and receiver.
        // in-flight messages should still be processed, even if the sender disconneccts
        _update: Arc<Lifeline>,
    }

    impl<T> Sender<T> {
        pub(crate) fn new(tx: mpsc::Sender<Subscription<T>>, update: Lifeline) -> Self {
            Self {
                tx,
                _update: Arc::new(update),
            }
        }

        pub async fn send(
            &mut self,
            subscription: Subscription<T>,
        ) -> Result<(), mpsc::error::SendError<Subscription<T>>> {
            self.tx.send(subscription).await
        }

        pub fn try_send(
            &mut self,
            subscription: Subscription<T>,
        ) -> Result<(), mpsc::error::TrySendError<Subscription<T>>> {
            self.tx.try_send(subscription)
        }
    }

    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            Self {
                tx: self.tx.clone(),
                _update: self._update.clone(),
            }
        }
    }

    impl_channel_clone!(Sender<T>);

    #[async_trait]
    impl<T> crate::Sender<Subscription<T>> for Sender<T>
    where
        T: Debug + Send + Sync,
    {
        async fn send(
            &mut self,
            value: Subscription<T>,
        ) -> Result<(), LifelineSendError<Subscription<T>>> {
            self.tx
                .send(value)
                .await
                .map_err(|err| LifelineSendError::Return(err.0))
        }
    }

    /// A receiver which provides `SubscriptionState` messages.  Returns immediately on the first `recv()`, and then waits for subscription updates.
    /// Also provides `.contains(id)` and `.get_identifier(id)` utility methods, which are non-blocking.
    #[derive(Debug)]
    pub struct Receiver<T> {
        rx: watch::Receiver<SubscriptionState<T>>,
    }

    impl<T> Receiver<T> {
        pub fn new(rx: watch::Receiver<SubscriptionState<T>>) -> Self {
            Self { rx }
        }
    }

    impl<T: Hash + Eq> Receiver<T> {
        /// Returns true if the subscription channel is currently subscribed to the given identifier
        pub fn contains(&self, id: &T) -> bool {
            self.rx.borrow().contains(id)
        }

        /// Returns a unique index for the subscription on the given identifier, or None if the channel is not subscribed to the identifier.
        pub fn get_identifier(&self, id: &T) -> Option<usize> {
            self.rx.borrow().get(id)
        }
    }

    impl<T: Clone> Receiver<T> {
        pub async fn recv(&mut self) -> Option<SubscriptionState<T>> {
            if let Err(_) = self.rx.changed().await {
                return None;
            }

            Some(self.rx.borrow().clone())
        }
    }

    impl<T: Hash + Eq + Clone> Receiver<T> {
        #[allow(dead_code)]
        fn iter(&self) -> impl Iterator<Item = T> {
            let items = self.rx.borrow().subscriptions.clone();
            items.into_iter().map(|(k, _v)| k)
        }
    }

    impl<T> Clone for Receiver<T> {
        fn clone(&self) -> Self {
            Self {
                rx: self.rx.clone(),
            }
        }
    }

    impl_channel_clone!(Receiver<T>);

    #[async_trait]
    impl<T> crate::Receiver<SubscriptionState<T>> for Receiver<T>
    where
        T: Clone + Debug + Send + Sync,
    {
        async fn recv(&mut self) -> Option<SubscriptionState<T>> {
            self.rx.recv().await
        }
    }
}

mod messages {
    use std::{collections::HashMap, hash::Hash};

    /// Subscribes, or unsubscribes to the given identifier
    #[derive(Debug, Clone)]
    pub enum Subscription<T> {
        Subscribe(T),
        Unsubscribe(T),
    }

    /// Represents the current subscription state of the channel.
    #[derive(Clone, Debug)]
    pub struct SubscriptionState<T> {
        pub subscriptions: HashMap<T, usize>,
    }

    impl<T: Hash + Eq> SubscriptionState<T> {
        /// Returns true if the subscription channel is currently subscribed to the given identifier
        pub fn contains(&self, id: &T) -> bool {
            self.subscriptions.contains_key(id)
        }

        /// Returns a unique index for the subscription on the given identifier, or None if the channel is not subscribed to the identifier.
        pub fn get(&self, id: &T) -> Option<usize> {
            self.subscriptions.get(id).copied()
        }
    }

    impl<T> Default for SubscriptionState<T>
    where
        T: Hash + PartialEq,
    {
        fn default() -> Self {
            Self {
                subscriptions: HashMap::new(),
            }
        }
    }
}

mod service {
    use super::messages::{Subscription, SubscriptionState};
    use crate::Task;
    use crate::{Bus, Lifeline, Service};
    use std::{fmt::Debug, hash::Hash, marker::PhantomData};

    pub struct UpdateService<T> {
        _t: PhantomData<T>,
    }

    impl<T> Service for UpdateService<T>
    where
        T: Clone + Hash + Eq + Debug + Send + Sync + 'static,
    {
        type Bus = super::bus::SubscriptionBus<T>;
        type Lifeline = anyhow::Result<Lifeline>;

        fn spawn(bus: &Self::Bus) -> Self::Lifeline {
            let mut rx = bus.rx::<Subscription<T>>()?.into_inner();
            let tx = bus.tx::<SubscriptionState<T>>()?.into_inner();
            let mut next_id = 0usize;
            let lifeline = Self::try_task("run", async move {
                let mut state = SubscriptionState::default();
                while let Some(msg) = rx.recv().await {
                    match msg {
                        Subscription::Subscribe(id) => {
                            if state.subscriptions.contains_key(&id) {
                                continue;
                            }

                            state.subscriptions.insert(id, next_id);
                            tx.send(state.clone())?;
                            next_id += 1;
                        }
                        Subscription::Unsubscribe(id) => {
                            if !state.subscriptions.contains_key(&id) {
                                continue;
                            }

                            state.subscriptions.remove(&id);
                            tx.send(state.clone())?;
                        }
                    }
                }

                Ok(())
            });

            Ok(lifeline)
        }
    }
}
