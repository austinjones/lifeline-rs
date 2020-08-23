mod macros;
mod slot;
mod storage;

use crate::{
    bus::{Link, Message, Resource},
    error::{type_name, AlreadyLinkedError, TakeChannelError, TakeResourceError},
    Bus, Channel, Storage,
};

use log::debug;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    sync::{RwLock, RwLockWriteGuard},
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
    fn rx<Msg>(&self) -> Result<<Msg::Channel as Channel>::Rx, TakeChannelError>
    where
        Msg: crate::bus::Message<Self> + 'static,
    {
        self.storage().link_channel::<Msg, Self>();
        self.storage().clone_rx::<Msg, Self>()
    }

    fn tx<Msg>(&self) -> Result<<Msg::Channel as Channel>::Tx, TakeChannelError>
    where
        Msg: crate::bus::Message<Self> + 'static,
    {
        self.storage().link_channel::<Msg, Self>();
        self.storage().clone_tx::<Msg, Self>()
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
