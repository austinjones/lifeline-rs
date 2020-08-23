use crate::{
    bus::{Link, Message, Resource},
    error::{type_name, AlreadyLinkedError, TakeChannelError, TakeResourceError},
    Bus, Channel, Storage,
};

use super::{slot::BusSlot, DynBus};
use log::debug;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    sync::{RwLock, RwLockWriteGuard},
};
#[derive(Debug)]
pub struct DynBusStorage<B> {
    state: RwLock<DynBusState>,
    _bus: PhantomData<B>,
}

#[derive(Debug)]
struct DynBusState {
    pub(crate) channels: HashSet<TypeId>,
    pub(crate) capacity: HashMap<TypeId, usize>,
    pub(crate) tx: HashMap<TypeId, BusSlot>,
    pub(crate) rx: HashMap<TypeId, BusSlot>,
    pub(crate) resources: HashMap<TypeId, BusSlot>,
}

impl Default for DynBusState {
    fn default() -> Self {
        DynBusState {
            channels: HashSet::new(),
            capacity: HashMap::new(),
            tx: HashMap::new(),
            rx: HashMap::new(),
            resources: HashMap::new(),
        }
    }
}
impl<B: Bus> Default for DynBusStorage<B> {
    fn default() -> Self {
        DynBusStorage {
            state: RwLock::new(DynBusState::default()),
            _bus: PhantomData,
        }
    }
}

impl<B: Bus> DynBusStorage<B> {
    pub fn link_channel<Msg, Bus>(&self)
    where
        Msg: Message<B> + 'static,
    {
        let id = TypeId::of::<Msg>();

        if let Some(mut state) = self.try_lock(id) {
            let capacity = state
                .capacity
                .get(&id)
                .copied()
                .unwrap_or(Msg::Channel::default_capacity());

            let (tx, rx) = Msg::Channel::channel(capacity);

            debug!("{} linked in {}", type_name::<Msg>(), type_name::<Bus>());
            state.rx.insert(id, BusSlot::new(Some(rx)));
            state.tx.insert(id, BusSlot::new(Some(tx)));

            state.channels.insert(id);
        }
    }

    pub fn clone_rx<Msg, Bus>(&self) -> Result<<Msg::Channel as Channel>::Rx, TakeChannelError>
    where
        Msg: Message<B> + 'static,
    {
        self.link_channel::<Msg, Bus>();

        let id = TypeId::of::<Msg>();

        let mut state = self.state.write().unwrap();
        let state = &mut *state;
        let tx = &state.tx;
        let rx = &mut state.rx;

        let tx = tx
            .get(&id)
            .map(|slot| slot.get_tx::<Msg::Channel>())
            .flatten();

        let slot = rx
            .get_mut(&id)
            .ok_or_else(|| TakeChannelError::partial_take::<Bus, Msg>(Link::Rx))?;

        slot.clone_rx::<Msg::Channel>(tx)
            .ok_or_else(|| TakeChannelError::already_taken::<Bus, Msg>(Link::Tx))
    }

    pub fn clone_tx<Msg, Bus>(&self) -> Result<<Msg::Channel as Channel>::Tx, TakeChannelError>
    where
        Msg: Message<B> + 'static,
    {
        self.link_channel::<Msg, Bus>();

        let id = TypeId::of::<Msg>();

        let mut state = self.state.write().unwrap();
        let senders = &mut state.tx;

        // if the channel is linked, but the slot is empty,
        // this means the user used take_rx, but asked for tx
        let slot = senders
            .get_mut(&id)
            .ok_or_else(|| TakeChannelError::partial_take::<Bus, Msg>(Link::Tx))?;

        slot.clone_tx::<Msg::Channel>()
            .ok_or_else(|| TakeChannelError::already_taken::<Bus, Msg>(Link::Tx))
    }

    pub fn clone_resource<Res>(&self) -> Result<Res, TakeResourceError>
    where
        Res: Resource<B> + 'static,
    {
        let id = TypeId::of::<Res>();

        let mut state = self.state.write().unwrap();
        let resources = &mut state.resources;
        let slot = resources
            .get_mut(&id)
            .ok_or_else(|| TakeResourceError::uninitialized::<Self, Res>())?;

        slot.clone_storage::<Res>()
            .ok_or_else(|| TakeResourceError::taken::<Self, Res>())
    }

    pub fn store_resource<Res: Send + 'static, Bus>(&self, value: Res) {
        let id = TypeId::of::<Res>();

        let mut state = self.state.write().unwrap();
        let resources = &mut state.resources;

        if !resources.contains_key(&id) {
            resources.insert(id.clone(), BusSlot::empty::<Res>());
        }

        debug!("{} stored in {}", type_name::<Res>(), type_name::<Bus>());

        let slot = resources.get_mut(&id).unwrap();

        slot.put(value);
    }

    pub fn store_channel<Msg, Chan, Bus>(
        &self,
        rx: Option<Chan::Rx>,
        tx: Option<Chan::Tx>,
    ) -> Result<(), AlreadyLinkedError>
    where
        Chan: Channel,
        Msg: 'static,
    {
        if rx.is_none() && tx.is_none() {
            return Ok(());
        }

        let id = TypeId::of::<Msg>();

        let mut target = self.state.write().expect("cannot lock other");
        if target.channels.contains(&id) {
            return Err(AlreadyLinkedError::new::<Self, Msg>());
        }

        let link = match (rx.is_some(), tx.is_some()) {
            (true, true) => Link::Both,
            (true, false) => Link::Rx,
            (false, true) => Link::Tx,
            (false, false) => unreachable!(),
        };

        debug!(
            "{}/{} stored in {}",
            type_name::<Msg>(),
            link,
            type_name::<Bus>(),
        );

        target.channels.insert(id);
        target.tx.insert(id.clone(), BusSlot::new(tx));
        target.rx.insert(id.clone(), BusSlot::new(rx));

        Ok(())
    }

    // pub fn take_channel<Msg, Source, Target, Chan>(
    //     &self,
    //     other: &Source,
    //     rx: bool,
    //     tx: bool,
    // ) -> Result<(), crate::bus::TakeChannelError>
    // where
    //     Msg: Message<Target, Channel = Chan> + Message<Source, Channel = Chan> + 'static,
    //     Chan: Channel,
    //     Source: DynBus,
    // {
    //     // TODO: clean up this function.  way too complicated
    //     if !rx && !tx {
    //         return Ok(());
    //     }

    //     other.storage().link_channel::<Msg, Source>();

    //     let id = TypeId::of::<Msg>();

    //     let mut target = self.state.write().expect("cannot lock other");
    //     if target.channels.contains(&id) {
    //         return Err(TakeChannelError::already_linked::<Target, Msg>());
    //     }

    //     let (rx_value, tx_value) = {
    //         let source = other.storage();
    //         let mut source = source.state.write().expect("cannot lock source");

    //         let tx_value = if tx {
    //             source
    //                 .tx
    //                 .get_mut(&id)
    //                 .map(|v| v.clone_tx::<Chan>())
    //                 .flatten()
    //         } else {
    //             None
    //         };

    //         let rx_value = if rx {
    //             source
    //                 .rx
    //                 .get_mut(&id)
    //                 .map(|v| v.clone_rx::<Chan>(tx_value.as_ref()))
    //                 .flatten()
    //         } else {
    //             None
    //         };

    //         (rx_value, tx_value)
    //     };

    //     let rx_missing = rx && rx_value.is_none();
    //     let tx_missing = tx && tx_value.is_none();
    //     match (rx_missing, tx_missing) {
    //         (true, true) => {
    //             return Err(TakeChannelError::already_taken::<Source, Msg>(Link::Both));
    //         }
    //         (true, false) => {
    //             return Err(TakeChannelError::already_taken::<Source, Msg>(Link::Rx));
    //         }
    //         (false, true) => {
    //             return Err(TakeChannelError::already_taken::<Source, Msg>(Link::Tx));
    //         }
    //         _ => {}
    //     }

    //     let link = match (rx && rx_value.is_some(), tx && tx_value.is_some()) {
    //         (true, true) => Link::Both,
    //         (true, false) => Link::Rx,
    //         (false, true) => Link::Tx,
    //         (false, false) => unreachable!(),
    //     };

    //     target.channels.insert(id);
    //     debug!(
    //         "{}/{} moved: {} => {}",
    //         type_name::<Msg>(),
    //         link,
    //         type_name::<Source>(),
    //         type_name::<Target>()
    //     );

    //     if rx {
    //         target.rx.insert(id.clone(), BusSlot::new(rx_value));
    //     }

    //     if tx {
    //         target.tx.insert(id.clone(), BusSlot::new(tx_value));
    //     }

    //     Ok(())
    // }

    pub fn take_resource<Res, Source, Target>(
        &self,
        other: &Source,
    ) -> Result<(), TakeResourceError>
    where
        Res: Resource<Source>,
        Res: Resource<Target>,
        Res: Storage,
        Source: DynBus,
    {
        let id = TypeId::of::<Res>();

        let mut target = self.state.write().expect("cannot lock other");
        if target.resources.contains_key(&id) {
            return Err(TakeResourceError::taken::<Source, Res>());
        }

        let source = other.storage();

        let res = source.clone_resource::<Res>()?;
        drop(source);

        debug!(
            "Resource {} moved: {} => {}",
            type_name::<Res>(),
            type_name::<Source>(),
            type_name::<Target>()
        );
        target.resources.insert(id.clone(), BusSlot::new(Some(res)));

        Ok(())
    }

    pub fn capacity<Msg>(&self, capacity: usize) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<B> + 'static,
    {
        let id = TypeId::of::<Msg>();

        let state = self.state.read().unwrap();

        if state.capacity.contains_key(&id) {
            return Err(AlreadyLinkedError::new::<B, Msg>());
        }

        drop(state);

        let mut state = self.state.write().unwrap();

        state.capacity.insert(id, capacity);

        Ok(())
    }

    fn try_lock(&self, id: TypeId) -> Option<RwLockWriteGuard<DynBusState>> {
        let state = self.state.read().unwrap();
        if state.channels.contains(&id) {
            return None;
        }

        drop(state);

        let state = self.state.write().unwrap();
        if state.channels.contains(&id) {
            return None;
        }

        Some(state)
    }
}
