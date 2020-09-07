//! The DynBus implementation used by `lifeline_bus!`, and TypeId-based slot storage.
mod macros;
mod slot;
mod storage;

use crate::{
    bus::{Message, Resource},
    channel::lifeline::{receiver::LifelineReceiver, sender::LifelineSender},
    error::{AlreadyLinkedError, TakeChannelError, TakeResourceError},
    Bus, Channel,
};

pub use storage::DynBusStorage;

/// An extension trait which defines operations on a DynBus, which stores `box dyn` trait objects internally.
///
/// DynBus implementations are created using the `lifeline_bus!` macro.
pub trait DynBus: Bus {
    /// Stores an manually constructed Receiver on the bus, for the provided message type.
    ///
    /// This is useful if you need to link a lifeline bus to other async message-based code.
    ///
    /// If the message channel has already been linked (from a call to `bus.rx`, `bus.tx`, or `bus.capacity`), returns an error.
    fn store_rx<Msg>(&self, rx: <Msg::Channel as Channel>::Rx) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static;

    /// Stores an manually constructed Sender on the bus, for the provided message type.
    ///
    /// This is useful if you need to link a lifeline bus to other async message-based code.
    ///
    /// If the message channel has already been linked (from a call to `bus.rx`, `bus.tx`, or `bus.capacity`), returns an error.
    fn store_tx<Msg>(&self, tx: <Msg::Channel as Channel>::Tx) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static;

    /// Stores a channel pair on the bus, for the provided message type.
    ///
    /// If the message channel has already been linked (from a call to `bus.rx`, `bus.tx`, or `bus.capacity`), returns an error.
    fn store_channel<Msg>(
        &self,
        rx: <Msg::Channel as Channel>::Rx,
        tx: <Msg::Channel as Channel>::Tx,
    ) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static;

    /// Stores a resource on the bus.
    ///
    /// Resources are commonly used for clonable configuration structs, or takeable resources such as websocket connections.
    fn store_resource<R: Resource<Self>>(&self, resource: R);

    /// Returns the `DynBusStorage` struct which manages the trait object slots.
    fn storage(&self) -> &DynBusStorage<Self>;
}

impl<T> Bus for T
where
    T: DynBus,
{
    fn rx<Msg>(
        &self,
    ) -> Result<LifelineReceiver<Msg, <Msg::Channel as Channel>::Rx>, TakeChannelError>
    where
        Msg: crate::bus::Message<Self> + 'static,
    {
        self.storage().link_channel::<Msg, Self>();
        let rx = self.storage().clone_rx::<Msg, Self>()?;
        Ok(LifelineReceiver::new(rx))
    }

    fn tx<Msg>(
        &self,
    ) -> Result<LifelineSender<Msg, <Msg::Channel as Channel>::Tx>, TakeChannelError>
    where
        Msg: crate::bus::Message<Self> + 'static,
    {
        self.storage().link_channel::<Msg, Self>();
        let tx = self.storage().clone_tx::<Msg, Self>()?;
        Ok(LifelineSender::new(tx))
    }

    fn capacity<Msg>(&self, capacity: usize) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static,
    {
        self.storage().capacity::<Msg>(capacity)
    }

    fn resource<Res>(&self) -> Result<Res, TakeResourceError>
    where
        Res: Resource<Self>,
    {
        self.storage().clone_resource::<Res>()
    }
}
