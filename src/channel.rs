use crate::Storage;

mod futures;
pub mod lifeline;
// pub mod historical;
pub mod subscription;
mod tokio;

pub trait Channel {
    type Tx: Storage + Send + 'static;
    type Rx: Storage + Send + 'static;

    fn channel(capacity: usize) -> (Self::Tx, Self::Rx);

    fn default_capacity() -> usize;

    fn clone_tx(tx: &mut Option<Self::Tx>) -> Option<Self::Tx> {
        Self::Tx::take_or_clone(tx)
    }

    fn clone_rx(rx: &mut Option<Self::Rx>, _tx: Option<&Self::Tx>) -> Option<Self::Rx> {
        Self::Rx::take_or_clone(rx)
    }
}
