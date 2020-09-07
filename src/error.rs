//! All the lifeline error types.

use crate::Link;
use regex::Regex;
use std::fmt::Debug;
use thiserror::Error;

/// Utility function which turns an error into it's debug message as an anyhow::Error.
pub fn into_msg<Err: Debug>(err: Err) -> anyhow::Error {
    let message = format!("{:?}", err);
    anyhow::Error::msg(message)
}

/// An error produced when calling `lifeline::Sender::send`
#[derive(Error, Debug, PartialEq)]
#[error("send error: ")]
pub enum SendError<T: Debug> {
    /// The channel has been closed, and the value is returned
    #[error("channel closed, message: {0:?}")]
    Return(T),

    /// The channel has been closed, but no value was returned
    #[error("channel closed")]
    Closed,
}

pub(crate) fn type_name<T>() -> String {
    let name = std::any::type_name::<T>();

    let regex = Regex::new("[a-z][A-Za-z0-9_]+::").expect("Regex compiles");
    regex.replace_all(name, "").to_string()
}

/// An error produced when attempting to take a Sender or Receiver from the bus.
#[derive(Error, Debug)]
pub enum TakeChannelError {
    /// The channel was partially linked on the bus, and this endpoint was not set.
    #[error("channel endpoints partially taken: {0}")]
    PartialTake(NotTakenError),

    /// The channel was already linked, and the requested operation required a new channel endpoint
    #[error("channel already linked: {0}")]
    AlreadyLinked(AlreadyLinkedError),

    /// The channel endpoint is not clonable, and the link was already taken
    #[error("channel already taken: {0}")]
    AlreadyTaken(LinkTakenError),
}

impl TakeChannelError {
    pub fn partial_take<Bus, Msg>(link: Link) -> Self {
        Self::PartialTake(NotTakenError::new::<Bus, Msg>(link))
    }

    pub fn already_linked<Bus, Msg>() -> Self {
        Self::AlreadyLinked(AlreadyLinkedError::new::<Bus, Msg>())
    }

    pub fn already_taken<Bus, Msg>(link: Link) -> Self {
        Self::AlreadyTaken(LinkTakenError::new::<Bus, Msg>(link))
    }
}

/// The described endpoint could not be taken from the bus
#[derive(Error, Debug)]
#[error("endpoint not taken: {bus} < {message}::{link} >")]
pub struct NotTakenError {
    pub bus: String,
    pub message: String,
    pub link: Link,
}

impl NotTakenError {
    pub fn new<Bus, Message>(link: Link) -> Self {
        NotTakenError {
            bus: type_name::<Bus>().to_string(),
            message: type_name::<Message>().to_string(),
            link,
        }
    }
}

/// The described endpoint was already taken from the bus
#[derive(Error, Debug)]
#[error("link already taken: {bus} < {message}::{link} >")]
pub struct LinkTakenError {
    pub bus: String,
    pub message: String,
    pub link: Link,
}

impl LinkTakenError {
    pub fn new<Bus, Message>(link: Link) -> Self {
        LinkTakenError {
            bus: type_name::<Bus>().to_string(),
            message: type_name::<Message>().to_string(),
            link,
        }
    }
}

/// The channel was already linked on the bus, but the operation required the creation of a new endpoint pair.
#[derive(Error, Debug)]
#[error("link already generated: {bus} < {message} >")]
pub struct AlreadyLinkedError {
    pub bus: String,
    pub message: String,
}

impl AlreadyLinkedError {
    pub fn new<Bus, Message>() -> Self {
        AlreadyLinkedError {
            bus: type_name::<Bus>().to_string(),
            message: type_name::<Message>().to_string(),
        }
    }
}

/// The resource was not initialized, or was not clonable and was already taken
#[derive(Error, Debug)]
pub enum TakeResourceError {
    /// The resource was uninitialized
    #[error("{0}")]
    Uninitialized(ResourceUninitializedError),

    /// The resource was not clonable, and had already been taken
    #[error("{0}")]
    Taken(ResourceTakenError),
}

impl TakeResourceError {
    pub fn uninitialized<Bus, Res>() -> Self {
        Self::Uninitialized(ResourceUninitializedError::new::<Bus, Res>())
    }

    pub fn taken<Bus, Res>() -> Self {
        Self::Taken(ResourceTakenError::new::<Bus, Res>())
    }
}

/// The resource was already taken from the bus
#[derive(Error, Debug)]
#[error("resource already taken: {bus} < {resource} >")]
pub struct ResourceTakenError {
    pub bus: String,
    pub resource: String,
}

impl ResourceTakenError {
    pub fn new<Bus, Res>() -> Self {
        ResourceTakenError {
            bus: type_name::<Bus>().to_string(),
            resource: type_name::<Res>().to_string(),
        }
    }
}

/// The resource was uninitialized on the bus
#[derive(Error, Debug)]
#[error("resource uninitialized: {bus} < {resource} >")]
pub struct ResourceUninitializedError {
    pub bus: String,
    pub resource: String,
}

impl ResourceUninitializedError {
    pub fn new<Bus, Res>() -> Self {
        ResourceUninitializedError {
            bus: type_name::<Bus>().to_string(),
            resource: type_name::<Res>().to_string(),
        }
    }
}

/// The resource was already initialized on the bus, and the operation required an uninitialized resource
#[derive(Error, Debug)]
#[error("resource already initialized: {bus} < {resource} >")]
pub struct ResourceInitializedError {
    pub bus: String,
    pub resource: String,
}

impl ResourceInitializedError {
    pub fn new<Bus, Res>() -> Self {
        ResourceInitializedError {
            bus: type_name::<Bus>().to_string(),
            resource: type_name::<Res>().to_string(),
        }
    }
}
