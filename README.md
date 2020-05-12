# Core Bluetooth

Safe wrapper around [Core Bluetooth framework](https://developer.apple.com/documentation/corebluetooth)
used to communicate with Bluetooth-equipped low energy (LE) and Basic Rate / Enhanced Data Rate
(BR/EDR) wireless technology.

Currently only the central role is supported.

## Usage

See example in the [crate docs](https://docs.rs/core_bluetooth/#example) and also the `examples` directory.

## Crate Features

By default MPSC rendezvous channel from `std` is used to perform native framework calls. With `async_std_unstable` 
feature chis channel can be replaced with `async_std::sync::channel` making it possible to pump events in async context.
Note the `async_std` will need `unstable` feature enabled.