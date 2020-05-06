use log::*;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::process::exit;

use core_bluetooth::central::*;
use core_bluetooth::central::peripheral::Peripheral;
use core_bluetooth::*;
use core_bluetooth::uuid::Uuid;

const SERVICE: &str = "ebe0ccb0-7a0a-4b0c-8a1a-6ff2997da3a6";
const CHARACTERISTIC: &str = "ebe0ccc1-7a0a-4b0c-8a1a-6ff2997da3a6";

struct App {
    central: CentralManager,
    receiver: Receiver<CentralEvent>,
    connected_peripherals: HashSet<Peripheral>,
    uuid_to_short_id: HashMap<Uuid, u32>,
    prev_short_id: u32,
}

impl App {
    fn new() -> Self {
        let (central, receiver) = CentralManager::new();
        Self {
            central,
            receiver,
            connected_peripherals: HashSet::new(),
            uuid_to_short_id: HashMap::new(),
            prev_short_id: 0,
        }
    }

    fn handle_event(&mut self, event: CentralEvent) {
        debug!("new event: {:#?}", event);
        match event {
            CentralEvent::ManagerStateChanged { new_state } => {
                match new_state {
                    ManagerState::Unsupported => {
                        eprintln!("Bluetooth is not supported on this system");
                        exit(1);
                    },
                    ManagerState::Unauthorized => {
                        eprintln!("The app is not authorized to use Bluetooth on this system");
                        exit(1);
                    },
                    ManagerState::PoweredOff => {
                        eprintln!("Bluetooth is disabled, please enable it");
                    },
                    ManagerState::PoweredOn => {
                        info!("scanning for peripherals");
                        println!("Discovering Xiaomi sensors...");
                        self.central.get_peripherals_with_services(&[SERVICE.parse().unwrap()]);
                        self.central.scan();
                    },
                    _ => {},
                }
            }
            CentralEvent::PeripheralDiscovered {
                peripheral,
                advertisement_data,
                rssi,
            } => {
                if advertisement_data.is_connectable() != Some(false) &&
                    self.connected_peripherals.insert(peripheral.clone())
                {
                    info!("connecting to {} {} dB ({:?})",
                        peripheral.id(), rssi, advertisement_data.local_name());
                    self.central.connect(&peripheral);
                }
            }
            CentralEvent::GetPeripheralsWithServicesResult { peripherals, tag: _ } => {
                for p in peripherals {
                    if self.connected_peripherals.insert(p.clone()) {
                        debug!("connecting to {})", p.id());
                        self.central.connect(&p);
                    }
                }
            }
            CentralEvent::PeripheralConnected { peripheral } => {
                peripheral.discover_services_with_uuids(&[SERVICE.parse().unwrap()]);
            }
            CentralEvent::PeripheralDisconnected { peripheral, error: _, } => {
                self.connected_peripherals.remove(&peripheral);
                debug!("re-connecting to {})", peripheral.id());
                self.central.connect(&peripheral);
            }
            CentralEvent::PeripheralConnectFailed { peripheral, error } => {
                warn!("failed to connect to peripheral {}: {}",
                    peripheral.id(), error.map(|e| e.to_string()).unwrap_or_else(|| "<no error>".into()));
                self.central.connect(&peripheral);
            }
            CentralEvent::ServicesDiscovered { peripheral, services, } => {
                if let Ok(services) = services {
                    for service in services {
                        peripheral.discover_characteristics_with_uuids(&service, &[CHARACTERISTIC.parse().unwrap()]);
                    }
                }
            }
            CentralEvent::SubscriptionChanged { peripheral, characteristic: _, result } => {
                if result.is_err() {
                    error!("couldn't subscribe to characteristic of {}", peripheral.id());
                } else {
                    println!("Subscribed to {} (#{})", peripheral.id(), self.shorten_uuid(peripheral.id()));
                }
            }
            CentralEvent::CharacteristicsDiscovered { peripheral, service: _, characteristics } => {
                match characteristics {
                    Ok(chars) => {
                        info!("subscribing to characteristic {} of {}", chars[0].id(), peripheral.id());
                        peripheral.subscribe(&chars[0]);
                    }
                    Err(err) => error!("couldn't discover characteristics of {}: {}", peripheral.id(), err),
                }
            }
            CentralEvent::CharacteristicValue { peripheral, characteristic: _, value } => {
                if let Ok(value) = value {
                    let now = chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]");

                    let t = i16::from_le_bytes([value[0], value[1]]) as f64 / 100.0;
                    let rh = value[2];
                    println!("{} #{}: t = {} C, rh = {}%",
                        now, self.shorten_uuid(peripheral.id()), t, rh);
                }
            }
            _ => {}
        }
    }

    fn shorten_uuid(&mut self, uuid: Uuid) -> u32 {
        match self.uuid_to_short_id.entry(uuid) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                self.prev_short_id += 1;
                *e.insert(self.prev_short_id)
            }
        }
    }

    #[cfg(not(feature = "async_std_unstable"))]
    fn run(mut self) {
        debug!("running in std");
        while let Ok(event) = self.receiver.recv() {
            self.handle_event(event);
        }
    }

    #[cfg(feature = "async_std_unstable")]
    fn run(mut self) {
        debug!("running in async_std");
        async_std::task::block_on(async move {
            while let Some(event) = self.receiver.recv().await {
                self.handle_event(event);
            }
        })
    }
}

pub fn main() {
    env_logger::init();

    App::new().run();
}