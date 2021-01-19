//! Prelude, including all the traits and types required for typical lifeline usage.

pub use crate::{Bus, CarryFrom, CarryInto, Lifeline, Message, Resource, Service, Task};

#[cfg(feature = "dyn-bus")]
pub use crate::lifeline_bus;
