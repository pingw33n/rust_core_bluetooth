//! Reads and decodes packets received in advertisements.
//! Based on https://github.com/custom-components/sensor.mitemp_bt
//! Some devices encrypt the data they advertise. To decrypt a key must be extracted and specified
//! in `--key` argument. See https://github.com/custom-components/sensor.mitemp_bt/blob/master/faq.md#my-sensors-ble-advertisements-are-encrypted-how-can-i-get-the-key
//! on how to get the key.
use anyhow::*;
use enumflags2::BitFlags;
use log::*;
use macaddr::MacAddr6;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::process::exit;

use core_bluetooth::central::*;
use core_bluetooth::*;

const SERVICE: &str = "0000fe95-0000-1000-8000-00805f9b34fb";

struct App {
    central: CentralManager,
    receiver: Receiver<CentralEvent>,
    encryption_keys: HashMap<MacAddr6, Vec<u8>>,
    seen: HashSet<MacAddr6>,
}

impl App {
    fn new(encryption_keys: HashMap<MacAddr6, Vec<u8>>) -> Self {
        let (central, receiver) = CentralManager::new();
        Self {
            central,
            receiver,
            encryption_keys,
            seen: HashSet::new(),
        }
    }

    fn handle_event(&mut self, event: CentralEvent) {
        debug!("New event: {:#?}", event);
        match event {
            CentralEvent::ManagerStateChanged { new_state } => {
                match new_state {
                    ManagerState::Unsupported => {
                        error!("Bluetooth is not supported on this system");
                        exit(1);
                    },
                    ManagerState::Unauthorized => {
                        error!("The app is not authorized to use Bluetooth on this system");
                        exit(1);
                    },
                    ManagerState::PoweredOff => {
                        error!("Bluetooth is disabled, please enable it");
                    },
                    ManagerState::PoweredOn => {
                        info!("Discovering Xiaomi sensors...");
                        self.central.scan();
                    },
                    _ => {},
                }
            }
            CentralEvent::PeripheralDiscovered {
                advertisement_data,
                ..
            } => {
                if let Some(packet) = advertisement_data.service_data().get(SERVICE.parse().unwrap()) {
                    match Packet::parse(packet, |mac| self.encryption_keys.get(&mac).map(|v| &v[..])) {
                        Ok(packet) => {
                            if !packet.sensor_values.is_empty() {
                                info!("{} ({}): {:?}", packet.mac_addr, packet.device_kind, packet.sensor_values);
                            } else if self.seen.insert(packet.mac_addr) {
                                info!("New device: {} ({})", packet.mac_addr, packet.device_kind);
                            }
                        }
                        Err(e) => {
                            error!("Error parsing packet: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    #[cfg(not(feature = "async_std_unstable"))]
    fn run(mut self) {
        debug!("Running in std");
        while let Ok(event) = self.receiver.recv() {
            self.handle_event(event);
        }
    }

    #[cfg(feature = "async_std_unstable")]
    fn run(mut self) {
        debug!("Running in async_std");
        async_std::task::block_on(async move {
            while let Some(event) = self.receiver.recv().await {
                self.handle_event(event);
            }
        })
    }
}

#[derive(BitFlags, Copy, Clone, Debug)]
#[repr(u8)]
enum Flag {
    Encrypted   = 0x08,
    HasCap      = 0x20,
    HasData     = 0x40,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeviceKind {
    Hhccjcy01,
    Lywsdcgq,
    Lywsd02,
    Cgg1,
    Hhccpot002,
    Gcls002,
    Lywsd03mmc,
    Cgd1,
    Jqjcy01ym,
    Wx08zm,
}

impl DeviceKind {
    fn from_u16(v: u16) -> Option<Self> {
        use DeviceKind::*;
        Some(match v {
            0x9800 => Hhccjcy01,
            0xaa01 => Lywsdcgq,
            0x5b04 => Lywsd02,
            0x4703 => Cgg1,
            0x5d01 => Hhccpot002,
            0xbc03 => Gcls002,
            0x5b05 => Lywsd03mmc,
            0x7605 => Cgd1,
            0xdf02 => Jqjcy01ym,
            0x0a04 => Wx08zm,
            _ => return None,
        })
    }
}

impl std::fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_ascii_uppercase())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SensorValue {
    Battery(u8),
    Conductivity(u32),
    Consumable(u8),
    Formaldehyde(f32),
    Humidity(f32),
    Illuminance(u32),
    Moisture(u8),
    Switch(u8),
    Temperature(f32),
}


#[derive(Clone, Debug, PartialEq)]
struct Packet {
    mac_addr: MacAddr6,
    device_kind: DeviceKind,
    sensor_values: Vec<SensorValue>,
}

impl Packet {
    fn parse<'a, F>(packet: &[u8], get_encryption_key: F) -> Result<Self>
        where F: Fn(MacAddr6) -> Option<&'a [u8]>
    {
        debug!("parsing {} B packet: {}", packet.len(), hex::encode(packet));
        if packet.len() < 12 {
            return Err(anyhow!("packet is too small: {}", packet.len()));
        }

        let flags = BitFlags::from_bits_truncate(packet[0]);
        debug!("flags: {:?}", flags);

        let mut mac_addr = [0; 6];
        mac_addr.copy_from_slice(&packet[5..11]);
        mac_addr.reverse();
        let mac_addr = MacAddr6::from(mac_addr);
        debug!("mac: {}", mac_addr);

        let device_kind = u16::from_be_bytes([packet[2], packet[3]]);
        let device_kind = DeviceKind::from_u16(device_kind)
            .ok_or_else(|| anyhow!("unrecognized device type ({}): 0x{:x}", mac_addr, device_kind))?;
        debug!("device_kind: {}", device_kind);

        if !flags.contains(Flag::HasData) {
            debug!("no HasData flag");
            return Ok(Self {
                mac_addr,
                device_kind,
                sensor_values: vec![],
            });
        }

        let payload_start = if flags.contains(Flag::HasCap) { 12 } else { 11 };

        let payload = if flags.contains(Flag::Encrypted) {
            let key = get_encryption_key(mac_addr)
                .ok_or_else(|| anyhow!("no encryption key found for {}", mac_addr))?;

            let mut nonce = [0; 12];
            nonce[..6].copy_from_slice(&packet[5..11]);
            nonce[6..9].copy_from_slice(&packet[2..5]);
            nonce[9..12].copy_from_slice(&packet[packet.len() - 7..packet.len() - 4]);

            let tag = &packet[packet.len() - 4..];
            let aad = &[0x11];

            let payload = decrypt_aes_128_ccm(&packet[payload_start..packet.len() - 7], &key, &nonce, tag, aad)?;
            Cow::Owned(payload)
        } else {
            Cow::Borrowed(&packet[payload_start..])
        };

        let mut payload = &payload[..];
        debug!("payload: {}", hex::encode(payload));
        let mut r = Vec::new();
        while !payload.is_empty() {
            if payload.len() < 3 {
                warn!("truncated value");
                break;
            }
            let kind = u16::from_be_bytes([payload[0], payload[1]]);
            let len = payload[2] as usize;
            payload = &payload[3..];
            if len > payload.len() {
                warn!("truncated value");
                break;
            }
            let v = &payload[..len];
            payload = &payload[len..];

            let mut decoded = true;
            match len {
                1 => match kind {
                    0xa10 => r.push(SensorValue::Battery(v[0])),
                    0x810 => r.push(SensorValue::Moisture(v[0])),
                    0x1210 => r.push(SensorValue::Switch(v[0])),
                    0x1310 => r.push(SensorValue::Consumable(v[0])),
                    _ => decoded = false,
                }
                3 => match kind {
                    0x710 => r.push(SensorValue::Illuminance(u32::from_le_bytes([v[0], v[1], v[2], 0]))),
                    _ => decoded = false,
                }
                2 => match kind {
                    0x610 => r.push(SensorValue::Humidity(u16::from_le_bytes([v[0], v[1]]) as f32 / 10.0)),
                    0x410 => r.push(SensorValue::Temperature(i16::from_le_bytes([v[0], v[1]]) as f32 / 10.0)),
                    0x910 => r.push(SensorValue::Conductivity(u16::from_le_bytes([v[0], v[1]]) as u32)),
                    0x1010 => r.push(SensorValue::Formaldehyde(u16::from_le_bytes([v[0], v[1]]) as f32 / 100.0)),
                    _ => decoded = false,
                }
                4 => match kind {
                    0xd10 => {
                        r.push(SensorValue::Temperature(i16::from_le_bytes([v[0], v[1]]) as f32 / 10.0));
                        r.push(SensorValue::Humidity(u16::from_le_bytes([v[2], v[3]]) as f32 / 10.0));
                    }
                    _ => decoded = false,
                }
                _ => decoded = false,
            }
            if !decoded {
                warn!("couldn't decode sensor value: kind={:x} value={}", kind, hex::encode(v));
            }
        }
        Ok(Self {
            mac_addr,
            device_kind,
            sensor_values: r,
        })
    }
}

fn decrypt_aes_128_ccm(ciphertext: &[u8], key: &[u8], nonce: &[u8], tag: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    // Unfortunately Rust OpenSSL wrapper doesn't work with non-standard AES CCM tags and there's no
    // safe alternative.
    // See https://github.com/sfackler/rust-openssl/issues/1237

    use openssl_sys::*;
    use std::ptr::{null, null_mut};
    use std::convert::TryInto;

    unsafe {
        let cipher = EVP_aes_128_ccm();

        let mut out_len = 0;
        let mut out = vec![0; ciphertext.len() + EVP_CIPHER_block_size(cipher) as usize];

        let ctx = EVP_CIPHER_CTX_new();

        // Select cipher
        EVP_DecryptInit_ex(ctx, cipher, null_mut(), null(), null());

        // Set nonce length
        EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_SET_IVLEN, nonce.len().try_into().unwrap(), null_mut());

        // Set expected tag value
        EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_SET_TAG,
                            tag.len().try_into().unwrap(), tag.as_ptr() as *mut _);

        // Specify key and noce
        EVP_DecryptInit_ex(ctx, null(), null_mut(), key.as_ptr(), nonce.as_ptr());

        // Set ciphertext length
        let ciphertext_len = ciphertext.len().try_into().unwrap();
        EVP_DecryptUpdate(ctx, null_mut(), &mut out_len, null(), ciphertext_len);

        // Set AAD
        EVP_DecryptUpdate(ctx, null_mut(), &mut out_len, aad.as_ptr(), aad.len().try_into().unwrap());

        // Decrypt plaintext, verify tag
        let r = EVP_DecryptUpdate(ctx, out.as_mut_ptr(), &mut out_len, ciphertext.as_ptr(), ciphertext_len);

        if r > 0 {
            out.truncate(out_len as usize);
            Ok(out)
        } else {
            Err(anyhow!("error decrypting"))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn decrypt_aes_128_ccm_() {
        let ciphertext = hex!("7f4258f2a8");
        let key = hex!("0f8fbcfc7d41c89c9b486b44e67be743");
        let nonce = hex!("9e03c038c1a45b0532000000");
        let tag = hex!("b3f39389");
        let aad = &[0x11];

        assert_eq!(decrypt_aes_128_ccm(&ciphertext, &key, &nonce, &tag, aad).unwrap(),
            b"\x06\x10\x02\xae\x01"[..].to_vec())
    }

    #[test]
    fn parse_() {
        let mut keys = HashMap::new();
        keys.insert(MacAddr6(hex!("a4c138c0039e")), hex!("0f8fbcfc7d41c89c9b486b44e67be743"));
        let parse = |packet| Packet::parse(packet, |mac| keys.get(&mac).map(|v| &v[..])).unwrap();

        let exp_packet = |sensor_values| Packet {
            mac_addr: MacAddr6(hex!("a4c138c0039e")),
            device_kind: DeviceKind::Lywsd03mmc,
            sensor_values,
        };

        assert_eq!(parse(&hex!("58585b05329e03c038c1a47f4258f2a8000000b3f39389")),
            exp_packet(vec![SensorValue::Humidity(43.0)]));
        assert_eq!(parse(&hex!("0201060f1695fe30585b056e9e03c038c1a408")),
            exp_packet(vec![]));
    }
}

pub fn main() -> Result<()> {
    env_logger::from_env(env_logger::Env::default()
        .default_filter_or("info")).init();

    use clap::Arg;
    let clapp = clap::App::new("Xiaomi Passive Sensor Reader")
        .about("Reads advertised sensor values from variety of Xiaomi BTLE devices")
        .arg(Arg::with_name("keys")
            .short('k')
            .long("key")
            .about("Sets encryption key for device in format --key=a4:c1:38:c0:03:9e=0f8fbcfc7d41c89c9b486b44e67be743")
            .takes_value(true)
            .multiple(true));
    let matches = clapp.get_matches();

    let mut keys = HashMap::new();
    for key in matches.values_of("keys").unwrap_or_default() {
        let (mac_addr, key) = Some(key.split("=")
            .collect::<Vec<_>>())
            .filter(|v| v.len() == 2 && v[1].len() == 32)
            .and_then(|v| {
                Some((v[0].parse::<MacAddr6>().ok()?, hex::decode(v[1]).ok()?))
            })
            .ok_or_else(|| anyhow!("invalid 'key' argument: {}", key))?;
        keys.insert(mac_addr, key);
    }

    App::new(keys).run();

    Ok(())
}