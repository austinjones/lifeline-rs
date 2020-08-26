use crate::{error::type_name, Channel, Storage};

use std::{any::Any, fmt::Debug};

pub(crate) struct BusSlot {
    name: String,
    value: Option<Box<dyn Any + Send>>,
}

impl Debug for BusSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self.value {
            Some(_) => format!("BusSlot<{}>::Some(_)", self.name.as_str()),
            None => format!("BusSlot<{}>::Empty", self.name.as_str()),
        };

        f.debug_struct(string.as_str()).finish()
    }
}

impl BusSlot {
    pub fn new<T: Send + 'static>(value: Option<T>) -> Self {
        Self {
            // TODO: think about this?  uses memory, but it's nice for debugging
            name: type_name::<T>(),
            value: value.map(|v| Box::new(v) as Box<dyn Any + Send>),
        }
    }

    pub fn empty<T>() -> Self {
        Self {
            name: type_name::<T>(),
            value: None,
        }
    }

    pub fn put<T: Send + 'static>(&mut self, value: T) {
        self.value = Some(Box::new(value))
    }

    pub fn get_tx<Chan>(&self) -> Option<&Chan::Tx>
    where
        Chan: Channel,
        Chan::Tx: Any + 'static,
    {
        self.value
            .as_ref()
            .map(|boxed| boxed.downcast_ref().unwrap())
    }

    pub fn clone_rx<Chan>(&mut self, tx: Option<&Chan::Tx>) -> Option<Chan::Rx>
    where
        Chan: Channel,
        Chan::Rx: Storage + Send + 'static,
    {
        let mut taken = self.value.take().map(Self::cast);
        let cloned = Chan::clone_rx(&mut taken, tx);
        self.value = taken.map(|value| Box::new(value) as Box<dyn Any + Send>);
        cloned
    }

    pub fn clone_tx<Chan>(&mut self) -> Option<Chan::Tx>
    where
        Chan: Channel,
        Chan::Tx: Storage + Send + 'static,
    {
        let mut taken = self.value.take().map(Self::cast);
        let cloned = Chan::clone_tx(&mut taken);
        self.value = taken.map(|value| Box::new(value) as Box<dyn Any + Send>);
        cloned
    }

    pub fn clone_storage<Res>(&mut self) -> Option<Res>
    where
        Res: Storage + Send + Any,
    {
        let mut taken = self.value.take().map(Self::cast);
        let cloned = Res::take_or_clone(&mut taken);
        self.value = taken.map(|value| Box::new(value) as Box<dyn Any + Send>);
        cloned
    }

    fn cast<T: 'static>(boxed: Box<dyn Any + Send>) -> T {
        *boxed
            .downcast::<T>()
            .expect("BusSlot should always have correct type")
    }
}
