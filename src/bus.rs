use crate::{
    error::{AlreadyLinkedError, TakeChannelError, TakeResourceError},
    Channel, Storage,
};

use std::fmt::{Debug, Display};

/// Attaches a channel to the [Bus](./trait.Bus.html), carrying `Self` as a message.
///
/// The Channel associated type should be the Sender of the channel which will carry this message.
///
/// Once implemented, the [bus.rx::\<Self\>()](trait.Bus.html#tymethod.rx), [bus.tx::\<Self\>()](trait.Bus.html#tymethod.tx), and [bus.capacity::\<Self\>()](trait.Bus.html#tymethod.capacity) methods can be called.
///
/// ## Example:
/// ```
/// use lifeline::prelude::*;
/// use tokio::sync::mpsc;
///
/// lifeline_bus!(pub struct ExampleBus);
///
/// #[derive(Debug)]
/// pub struct ExampleMessage;
///
/// impl Message<ExampleBus> for ExampleMessage {
///    type Channel = mpsc::Sender<Self>;
/// }
///
/// fn main() -> anyhow::Result<()> {
///     let bus = ExampleBus::default();
///     let mut tx = bus.tx::<ExampleMessage>()?;
///     Ok(())
/// }
/// ```
pub trait Message<Bus>: Debug {
    type Channel: Channel;
}

/// Attaches a resource to the [Bus](./trait.Bus.html).  This resource can accessed from the bus using [bus.resource::\<Self\>()](trait.Bus.html#tymethod.resource).
///
/// The resource must implement [Storage](./trait.Storage.html), which describes whether the resource is taken or cloned.
///
/// Lifeline provides helper macros: [impl_storage_take!(MyResource)](./macro.impl_storage_take.html) and [impl_storage_clone!(MyResource)](./macro.impl_storage_clone.html).
///
/// ## Example:
/// ```
/// use lifeline::prelude::*;
/// use lifeline::impl_storage_clone;
/// use tokio::sync::mpsc;
///
/// lifeline_bus!(pub struct ExampleBus);
///
/// #[derive(Clone, Debug)]
/// pub struct MyResource;
/// impl_storage_clone!(MyResource);
///
/// impl Resource<ExampleBus> for MyResource {}
/// ```
pub trait Resource<Bus>: Storage + Debug + Send {}

/// Stores and distributes channel endpoints ([Senders](./trait.Sender.html) and [Receivers](./trait.Receiver.html)), as well as [Resource](./trait.Resource.html) values.
///
/// The bus allows you to write loosely-coupled applications, with adjacent lifeline [Services](./trait.Service.html) that do not depend on each other.
///
/// Most Bus implementations are defined using the [lifeline_bus!](./macro.lifeline_bus.html) macro.
///
/// ## Example:
/// ```
/// use lifeline::lifeline_bus;
///
/// lifeline_bus!(pub struct ExampleBus);
/// ```
pub trait Bus: Default + Debug + Sized {
    /// Configures the channel capacity, if the linked channel implementation takes a capacity during initialization
    ///
    /// Returns an [AlreadyLinkedError](./error/struct.AlreadyLinkedError.html), if the channel has already been initalized from another call to `capacity`, `rx`, or `tx`.
    ///
    /// ## Example:
    /// ```
    /// use lifeline::prelude::*;
    /// use tokio::sync::mpsc;
    /// lifeline_bus!(pub struct ExampleBus);
    ///  
    /// #[derive(Debug)]
    /// struct ExampleMessage {}
    /// impl Message<ExampleBus> for ExampleMessage {
    ///     type Channel = mpsc::Sender<Self>;
    /// }
    ///
    /// fn main() {
    ///     let bus = ExampleBus::default();
    ///     bus.capacity::<ExampleMessage>(1024);
    ///     let rx = bus.rx::<ExampleMessage>();
    /// }
    fn capacity<Msg>(&self, capacity: usize) -> Result<(), AlreadyLinkedError>
    where
        Msg: Message<Self> + 'static;

    /// Takes (or clones) the channel [Receiver](./trait.Receiver.html).  The message type must implement [Message\<Bus\>](./trait.Message.html), which defines the channel type.
    ///
    /// Returns the [Receiver](./trait.Receiver.html), or a [TakeChannelError](./error/enum.TakeChannelError.html) if the channel endpoint is not clonable, and has already been taken.
    ///
    /// - For `mpsc` channels, the Receiver is taken.
    /// - For `broadcast` channels, the Receiver is cloned.
    /// - For `watch` channels, the Receiver is cloned.
    ///
    /// ## Example:
    /// ```
    /// use lifeline::prelude::*;
    /// use tokio::sync::mpsc;
    /// lifeline_bus!(pub struct ExampleBus);
    ///  
    /// #[derive(Debug)]
    /// struct ExampleMessage {}
    /// impl Message<ExampleBus> for ExampleMessage {
    ///     type Channel = mpsc::Sender<Self>;
    /// }
    ///
    /// fn main() {
    ///     let bus = ExampleBus::default();
    ///     let rx = bus.rx::<ExampleMessage>();
    /// }
    /// ```
    fn rx<Msg>(&self) -> Result<<Msg::Channel as Channel>::Rx, TakeChannelError>
    where
        Msg: Message<Self> + 'static;

    /// Takes (or clones) the channel [Sender](./trait.Sender.html).  The message type must implement [Message\<Bus\>](./trait.Message.html), which defines the channel type.
    ///
    /// Returns the sender, or a [TakeChannelError](./error/enum.TakeChannelError.html) if the channel endpoint is not clonable, and has already been taken.
    ///
    /// - For `mpsc` channels, the Sender is cloned.
    /// - For `broadcast` channels, the Sender is cloned.
    /// - For `watch` channels, the Sender is taken.
    ///
    /// ## Example:
    /// ```
    /// use lifeline::prelude::*;
    /// use tokio::sync::mpsc;
    /// lifeline_bus!(pub struct ExampleBus);
    ///  
    /// #[derive(Debug)]
    /// struct ExampleMessage {}
    /// impl Message<ExampleBus> for ExampleMessage {
    ///     type Channel = mpsc::Sender<Self>;
    /// }
    ///
    /// fn main() {
    ///     let bus = ExampleBus::default();
    ///     let tx = bus.tx::<ExampleMessage>();
    /// }
    /// ```
    fn tx<Msg>(&self) -> Result<<Msg::Channel as Channel>::Tx, TakeChannelError>
    where
        Msg: Message<Self> + 'static;

    /// Takes (or clones) the [Resource](./trait.Resource.html).
    ///
    /// Returns the resource, or a [TakeResourceError](./error/enum.TakeResourceError.html) if the resource is not clonable, and has already been taken.
    ///
    /// ## Example:
    /// ```
    /// use lifeline::prelude::*;
    /// use lifeline::impl_storage_clone;
    /// use tokio::sync::mpsc;
    /// lifeline_bus!(pub struct ExampleBus);
    ///
    /// #[derive(Debug, Clone)]
    /// struct ExampleResource {}
    /// impl_storage_clone!(ExampleResource);
    /// impl Resource<ExampleBus> for ExampleResource {}
    ///
    /// fn main() {
    ///     let bus = ExampleBus::default();
    ///     let resource = bus.resource::<ExampleResource>();
    /// }
    /// ```
    fn resource<Res>(&self) -> Result<Res, TakeResourceError>
    where
        Res: Resource<Self>;
}

/// Represents the Sender, Receiver, or Both.  Used in error types.
#[derive(Debug)]
pub enum Link {
    /// The Sender half of the channel
    Tx,
    /// The Receiver half of the channel
    Rx,
    /// Both the Sender and Receiver endpoints
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
