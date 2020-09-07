use crate::{
    bus::{Link, Message, Resource},
    error::{type_name, AlreadyLinkedError, TakeChannelError, TakeResourceError},
    Bus, Channel,
};

use super::slot::BusSlot;
use log::debug;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    sync::{RwLock, RwLockWriteGuard},
};
/// Dynamic bus storage based on trait object slots, for Senders, Receivers, and Resources.
///
/// Most values are stored as `HashMap<TypeId, BusSlot>`
#[derive(Debug)]
pub struct DynBusStorage<B> {
    state: RwLock<DynBusState>,
    _bus: PhantomData<B>,
}

/// The internal state for:
/// - channels, a set of message TypeIds for which the channel has been linked
/// - capacity, the map from messageTypeId to the overriden channel capacity
/// - tx, the map from message TypeId to the channel sender
/// - rx, the map from message TypeId to the channel receiver
/// - resource, the map from resource TypeId to the resource value
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
    /// Links a channel on the bus, locking the state and inserting BusSlots for the sender/receiver pair
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

    /// Takes or clones the channel receiver, using the `Channel` trait implementation.
    /// Returns an error if the endpoint cannot be taken.
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
            .ok_or_else(|| TakeChannelError::already_taken::<Bus, Msg>(Link::Rx))
    }

    /// Takes or clones the channel sender, using the `Channel` trait implementation.
    /// Returns an error if the endpoint cannot be taken.
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

    /// Takes or clones the resource, using the `Storage` trait implementation.
    /// Returns an error if the resource cannot be taken.
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

    /// Stores the resource on the bus, overwriting it if it already exists
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

    /// Stores the (Rx, Tx) pair, or either of them if Nones are provided.
    /// This consumes the bus slot for this message type.  Future calls to store on this message type will fail.
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

    /// Writes a capacity to the bus storage, for the given message type.
    /// Returns an error if the channel is already linked in the bus storage (as this capacity would do nothing).
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

    /// Attempts to lock the bus, and acquire the state for the given message TypeId.
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
