use log::*;
use std::collections::{HashMap, HashSet};
use std::process::exit;
use core_bluetooth::central::*;
use core_bluetooth::ManagerState;

pub fn main() {
    env_logger::init();

    let (central, events) = CentralManager::new();

    let mut connected_peripherals = HashSet::new();

    let mut short_id = 0;
    let mut uuid_to_short_id = HashMap::new();
    let mut shorten_uuid = |uuid| {
        *uuid_to_short_id.entry(uuid).or_insert_with(|| { short_id += 1; short_id })
    };

    let service_uuid = "ebe0ccb0-7a0a-4b0c-8a1a-6ff2997da3a6".parse().unwrap();
    let char_uuid = "ebe0ccc1-7a0a-4b0c-8a1a-6ff2997da3a6".parse().unwrap();

    for event in events.iter() {
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
                        central.get_peripherals_with_services(&[service_uuid]);
                        central.scan();
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
                    connected_peripherals.insert(peripheral.clone())
                {
                    info!("connecting to {} {} dB ({:?})",
                        peripheral.id(), rssi, advertisement_data.local_name());
                    central.connect(&peripheral);
                }
            }
            CentralEvent::GetPeripheralsWithServicesResult { peripherals, tag: _ } => {
                for p in peripherals {
                    if connected_peripherals.insert(p.clone()) {
                        debug!("connecting to {})", p.id());
                        central.connect(&p);
                    }
                }
            }
            CentralEvent::PeripheralConnected { peripheral } => {
                peripheral.discover_services_with_uuids(&[service_uuid]);
            }
            CentralEvent::PeripheralDisconnected { peripheral, error: _, } => {
                connected_peripherals.remove(&peripheral);
                debug!("re-connecting to {})", peripheral.id());
                central.connect(&peripheral);
            }
            CentralEvent::PeripheralConnectFailed { peripheral, error } => {
                warn!("failed to connect to peripheral {}: {}",
                    peripheral.id(), error.map(|e| e.to_string()).unwrap_or_else(|| "<no error>".into()));
                central.connect(&peripheral);
            }
            CentralEvent::ServicesDiscovered { peripheral, services, } => {
                if let Ok(services) = services {
                    for service in services {
                        peripheral.discover_characteristics_with_uuids(&service, &[char_uuid]);
                    }
                }
            }
            CentralEvent::SubscriptionChanged { peripheral, characteristic: _, result } => {
                if result.is_err() {
                    error!("couldn't subscribe to characteristic of {}", peripheral.id());
                } else {
                    println!("Subscribed to {} (#{})", peripheral.id(), shorten_uuid(peripheral.id()));
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
                        now, shorten_uuid(peripheral.id()), t, rh);
                }
            }
            _ => {}
        }
    }
}