use crate::Storage;

#[cfg(feature = "async-std-channels")]
mod async_std;

pub mod lifeline;

#[cfg(feature = "tokio-channels")]
pub mod subscription;

#[cfg(feature = "tokio-channels")]
mod tokio;

/// A channel's (Sender, Receiver) pair.  Defines how the bus constructs and retrieves the values.
///
/// Channel endpoints can either be taken, or cloned.  The `Channel` trait has default implementations that honor the
/// `Storage` trait implementation of channel endpoints.  However, in some cases (such as tokio broadcast channels) the tx & rx endpoints are both required to implement this trait.
pub trait Channel {
    /// The Sender half of the channel.  This is used in `Message` implementations to attach channels to a `Bus`.
    type Tx: Storage + Send + 'static;

    /// The Receiver half of the channel.  This is constructed when `bus.tx` or `bus.rx` is called, and is driven by the `Message` implementation for the message.
    type Rx: Storage + Send + 'static;

    /// Constructs a new `(Sender, Receiver)` pair.  If the channel is bounded, use the provided capacity.
    fn channel(capacity: usize) -> (Self::Tx, Self::Rx);

    /// If the channel is bounded, provide a default capacity hint.  Users can override this with `bus.capacity(usize)`
    fn default_capacity() -> usize;

    /// Clones the Self::Tx value, or takes it from the option if this endpoint is not cloneable.
    fn clone_tx(tx: &mut Option<Self::Tx>) -> Option<Self::Tx> {
        Self::Tx::take_or_clone(tx)
    }

    /// Clones the Self::Rx value, or takes it from the option if this endpoint is not cloneable.
    /// The Tx endpoint is also available, which is necessary to implement Channel for broadcast channels
    /// (where new receivers are created from a tx subscription call)
    fn clone_rx(rx: &mut Option<Self::Rx>, _tx: Option<&Self::Tx>) -> Option<Self::Rx> {
        Self::Rx::take_or_clone(rx)
    }
}
