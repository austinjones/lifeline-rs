//! Prelude, including all the traits and types required for typical lifeline usage.

pub use crate::{
    Bus, CarryFrom, CarryInto, Lifeline, Message, Receiver, Resource, Sender, Service, Task,
};

#[cfg(feature = "dyn-bus")]
pub use crate::lifeline_bus;
