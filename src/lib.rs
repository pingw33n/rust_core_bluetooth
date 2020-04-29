#![deny(dead_code)]
#![deny(non_snake_case)]
#![deny(unused_imports)]
#![deny(unused_must_use)]

#[macro_use]
mod macros;

pub mod central;
pub mod error;
mod platform;
mod sync;
pub mod uuid;
mod util;

use static_assertions::*;

pub use sync::Receiver;

/// Arbitrary data to associate with certain asynchronous API calls.
pub type Tag = Box<dyn std::any::Any + Send>;

assert_impl_all!(Tag: Send);
assert_not_impl_any!(Tag: Sync);

/// The possible states of a Core Bluetooth manager.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ManagerState {
    /// The manager’s state is unknown.
    Unknown = 0,

    /// A state that indicates the connection with the system service was momentarily lost.
    Resetting = 1,

    /// A state that indicates this device doesn’t support the Bluetooth low energy central or client role.
    Unsupported = 2,

    /// A state that indicates the application isn’t authorized to use the Bluetooth low energy role.
    Unauthorized = 3,

    /// A state that indicates Bluetooth is currently powered off.
    PoweredOff = 4,

    /// A state that indicates Bluetooth is currently powered on and available to use.
    PoweredOn = 5,
}

impl ManagerState {
    fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Unknown,
            1 => Self::Resetting,
            2 => Self::Unsupported,
            3 => Self::Unauthorized,
            4 => Self::PoweredOff,
            5 => Self::PoweredOn,
            _ => return None,
        })
    }
}
