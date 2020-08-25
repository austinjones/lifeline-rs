mod macros;
mod slot;
mod storage;

use crate::{
    bus::{Link, Message, Resource},
    channel::lifeline::{receiver::LifelineReceiver, sender::LifelineSender},
    error::{type_name, AlreadyLinkedError, TakeChannelError, TakeResourceError},
    Bus, Channel, Storage,
};

pub use storage::DynBusStorage;

pub trait DynBus: Bus {
    fn store_rx<Msg>(&self, rx: <Msg::Channel as Channel>::Rx) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static;

    fn store_tx<Msg>(&self, tx: <Msg::Channel as Channel>::Tx) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static;

    fn store_channel<Msg>(
        &self,
        rx: <Msg::Channel as Channel>::Rx,
        tx: <Msg::Channel as Channel>::Tx,
    ) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static;

    fn store_resource<R: Resource<Self>>(&self, resource: R);

    // fn take_channel<Msg, Source>(&self, other: &Source) -> Result<(), TakeChannelError>
    // where
    //     Msg: Message<Self> + 'static,
    //     Msg: Message<Source, Channel = <Msg as Message<Self>>::Channel>,
    //     Source: DynBus;

    // fn take_rx<Msg, Source>(&self, other: &Source) -> Result<(), TakeChannelError>
    // where
    //     Msg: Message<Self> + 'static,
    //     Msg: Message<Source, Channel = <Msg as Message<Self>>::Channel>,
    //     Source: DynBus;

    // fn take_tx<Msg, Source>(&self, other: &Source) -> Result<(), TakeChannelError>
    // where
    //     Msg: Message<Self> + 'static,
    //     Msg: Message<Source, Channel = <Msg as Message<Self>>::Channel>,
    //     Source: DynBus;

    // fn take_resource<Res, Source>(&self, other: &Source) -> Result<(), TakeResourceError>
    // where
    //     Res: Storage,
    //     Res: Resource<Source>,
    //     Res: Resource<Self>,
    //     Source: DynBus;

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
