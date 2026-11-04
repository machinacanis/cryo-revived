//! Wire types of the QQ Bot OpenAPI & gateway protocol.

mod guild;
mod keyboard;
mod message;
mod outgoing;
mod protocol;

pub use guild::*;
pub use keyboard::*;
pub use message::*;
pub use outgoing::*;
pub use protocol::{event, op, Intent, Payload};

pub(crate) fn is_zero<T: Default + PartialEq>(n: &T) -> bool {
    n == &T::default()
}
