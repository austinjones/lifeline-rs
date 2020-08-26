use crate::Link;
use regex::Regex;
use std::fmt::Debug;
use thiserror::Error;

pub fn into_msg<Err: Debug>(err: Err) -> anyhow::Error {
    let message = format!("{:?}", err);
    anyhow::Error::msg(message)
}

#[derive(Error, Debug, PartialEq)]
#[error("send error: ")]
pub enum SendError<T: Debug> {
    #[error("channel closed, message: {0:?}")]
    Return(T),

    #[error("channel closed")]
    Closed,
}

pub(crate) fn type_name<T>() -> String {
    let name = std::any::type_name::<T>();

    let regex = Regex::new("[a-z][A-Za-z0-9_]+::").expect("Regex compiles");
    regex.replace_all(name, "").to_string()
}

#[derive(Error, Debug)]
pub enum TakeChannelError {
    #[error("channel endpoints partially taken: {0}")]
    PartialTake(NotTakenError),
    #[error("channel already linked: {0}")]
    AlreadyLinked(AlreadyLinkedError),
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

//TODO: encode Bus and Link types
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

//TODO: encode Bus and Link types
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

#[derive(Error, Debug)]
pub enum TakeResourceError {
    #[error("{0}")]
    Uninitialized(ResourceUninitializedError),
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
