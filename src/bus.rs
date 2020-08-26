use crate::{
    channel::lifeline::{receiver::LifelineReceiver, sender::LifelineSender},
    error::{AlreadyLinkedError, TakeChannelError, TakeResourceError},
    Channel, Storage,
};

use std::fmt::{Debug, Display};

pub trait Message<Bus>: Debug {
    type Channel: Channel;
}

pub trait Resource<Bus>: Storage + Debug + Send {}

pub trait Serves<Msg> {}
impl<B, Msg> Serves<Msg> for B where Msg: Message<B> {}

pub trait Stores<Res> {}
impl<B, R> Stores<R> for B where R: Resource<B> {}

/// The bus carries
pub trait Bus: Default + Debug + Sized {
    /// Returns the receiver on the first call, and

    fn capacity<Msg>(&self, capacity: usize) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static;

    fn rx<Msg>(
        &self,
    ) -> Result<LifelineReceiver<Msg, <Msg::Channel as Channel>::Rx>, TakeChannelError>
    where
        Msg: Message<Self> + 'static;

    fn tx<Msg>(
        &self,
    ) -> Result<LifelineSender<Msg, <Msg::Channel as Channel>::Tx>, TakeChannelError>
    where
        Msg: Message<Self> + 'static;

    fn resource<Res>(&self) -> Result<Res, TakeResourceError>
    where
        Res: Resource<Self>;
}

#[derive(Debug)]
pub enum Link {
    Tx,
    Rx,
    Both,
}

impl Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Link::Tx => f.write_str("Tx"),
            Link::Rx => f.write_str("Rx"),
            Link::Both => f.write_str("Both"),
        }
    }
}
