//! Safe wrapper around [Core Bluetooth framework](https://developer.apple.com/documentation/corebluetooth)
//! used to communicate with Bluetooth-equipped low energy (LE) and Basic Rate / Enhanced Data Rate
//! (BR/EDR) wireless technology.
//!
//! The API closely resembles to the native API with some changes for consistency sake.
//! The main difference is that this API lacks most of the functions for accessing retained
//! state, for thread-safety reasons. If needed users can maintain the retained state via
//! information from events.
//!
//! # Central role
//!
//! Central role is when application acts as "central" and initiates discovery of and connections
//! to peripherals. The [`central`](central/index.html) package contains all the needed objects for
//! central role.
//!
//! ## Example
//!
//! The following example shows how to discover peripherals, services and characteristics,
//! connect to peripherals and subscribe to characteristics.
//!
//! ```no_run
//! use core_bluetooth::*;
//! use core_bluetooth::central::*;
//!
//! let (central, receiver) = CentralManager::new();
//!
//! let handle_event = |event| {
//!     match event {
//!         CentralEvent::ManagerStateChanged { new_state } => {
//!             match new_state {
//!                 // Must be in PoweredOn state.
//!                 ManagerState::PoweredOn => central.scan(),
//!                 _ => panic!("no bluetooth available"),
//!             }
//!         }
//!         CentralEvent::PeripheralDiscovered { peripheral, advertisement_data, .. } => {
//!             if advertisement_data.is_connectable() != Some(false) {
//!                 central.connect(&peripheral);
//!             }
//!         }
//!         CentralEvent::PeripheralConnected { peripheral } => {
//!             peripheral.discover_services_with_uuids(&[
//!                 "ebe0ccb0-7a0a-4b0c-8a1a-6ff2997da3a6".parse().unwrap()]);
//!         }
//!         CentralEvent::ServicesDiscovered { peripheral, services } => {
//!             if let Ok(services) = services {
//!                 for service in services {
//!                     peripheral.discover_characteristics_with_uuids(&service, &[
//!                         "ebe0ccc1-7a0a-4b0c-8a1a-6ff2997da3a6".parse().unwrap()]);
//!                 }
//!             }
//!         }
//!         CentralEvent::CharacteristicsDiscovered { peripheral, characteristics, .. } => {
//!             if let Ok(chars) = characteristics {
//!                 peripheral.subscribe(&chars[0]);
//!             }
//!         }
//!         CentralEvent::CharacteristicValue { peripheral, value, .. } => {
//!             if let Ok(value) = value {
//!                 // Decode the value.
//!                 // In this example the value comes from a Xiaomi temperature sensor.
//!                 let t = i16::from_le_bytes([value[0], value[1]]) as f64 / 100.0;
//!                 let rh = value[2];
//!                 println!("t = {} C, rh = {}%", t, rh);
//!             }
//!         }
//!         _ => {}
//!     }
//! };
#![cfg_attr(not(feature = "async_std_unstable"), doc =r#"
while let Ok(event) = receiver.recv() {
    handle_event(event);
}
"#)]
#![cfg_attr(feature = "async_std_unstable", doc =r#"
async_std::task::block_on(async move {
    while let Some(event) = receiver.recv().await {
        handle_event(event);
    }
})
"#)]
//! ```
//!
//! You can find more examples in the `examples` directory.
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

/// Arbitrary data to associate with asynchronous API call.
pub type Tag = Box<dyn std::any::Any + Send>;

assert_impl_all!(Tag: Send);
assert_not_impl_any!(Tag: Sync);

/// The possible states of a Core Bluetooth manager.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
