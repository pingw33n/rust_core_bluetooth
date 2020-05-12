use objc::*;
use objc::runtime::*;
use static_assertions::assert_impl_all;
use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::platform::*;
use std::str::FromStr;

const BASE_UUID_BYTES: [u8; 16] = [0, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0x80, 0x5F, 0x9B, 0x34, 0xFB];

/// Bluetooth-tailored UUID.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// Returns UUID with all bytes set to zero.
    pub const fn zeroed() -> Self {
        Self([0; 16])
    }

    /// Returns the Base UUID (`00000000-0000-1000-8000-00805F9B34FB`) as defined by the specs.
    pub const fn base() -> Self {
        Self(BASE_UUID_BYTES)
    }

    /// Constructs instance from the specified bytes.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Constructs instance from the specified slice of variable length.
    /// The supported lengths are 2 for `uuid16`, 4 for `uuid32` and 16 for a standard UUID.
    ///
    /// # Panics
    ///
    /// Panics if `bytes` length is not 2, 4 or 16.
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self(match bytes.len() {
            2 => {
                let mut r = BASE_UUID_BYTES;
                r[2] = bytes[0];
                r[3] = bytes[1];
                r
            }
            4 => {
                let mut r = BASE_UUID_BYTES;
                r[0] = bytes[0];
                r[1] = bytes[1];
                r[2] = bytes[2];
                r[3] = bytes[3];
                r
            }
            16 => {
                let mut r = [0; 16];
                r.copy_from_slice(bytes);
                r
            }
            _ => panic!("invalid slice len {}, expected 2, 4 or 16 bytes", bytes.len()),
        })
    }

    /// Returns inner bytes array.
    pub fn bytes(&self) -> [u8; 16] {
        self.0
    }

    /// Returns the shortest possible UUID that is equivalent of this UUID.
    pub fn shorten(&self) -> &[u8] {
        if self.0[4..] == BASE_UUID_BYTES[4..] {
            if self.0[0..2] == BASE_UUID_BYTES[0..2] {
                &self.0[2..4]
            } else {
                &self.0[..4]
            }
        } else {
            &self.0
        }
    }
}

assert_impl_all!(Uuid: Send, Sync);

impl Deref for Uuid {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Uuid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5], self.0[6], self.0[7],
            self.0[8], self.0[9], self.0[10], self.0[11],
            self.0[12], self.0[13], self.0[14], self.0[15])
    }
}

impl fmt::Debug for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Uuid({})", self)
    }
}

impl From<[u8; 16]> for Uuid {
    fn from(v: [u8; 16]) -> Self {
        Self::from_bytes(v)
    }
}

impl From<&[u8]> for Uuid {
    fn from(v: &[u8]) -> Self {
        Self::from_slice(v)
    }
}

impl FromStr for Uuid {
    type Err = UuidParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.as_bytes();
        if s.len() != 36 {
            return Err(UuidParseError(()));
        }
        const PARTS: [(usize, usize); 4] = [(8, 4), (13, 6), (18, 8), (23, 10)];
        if s[PARTS[0].0] != b'-'
            || s[PARTS[1].0] != b'-'
            || s[PARTS[2].0] != b'-'
            || s[PARTS[3].0] != b'-'
        {
            return Err(UuidParseError(()));
        }

        fn decode(src: &[u8], dst: &mut [u8]) -> Result<(), UuidParseError> {
            debug_assert_eq!(src.len() % 2, 0);
            debug_assert_eq!(dst.len(), src.len() / 2);

            fn dig(c: u8) -> Result<u8, UuidParseError> {
                Ok(match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    b'A'..=b'F' => c - b'A' + 10,
                    _ => return Err(UuidParseError(())),
                })
            }

            for (s, d) in src.chunks(2).zip(dst.iter_mut()) {
                *d = (dig(s[0])? << 4) | dig(s[1])?;
            }

            Ok(())
        }

        let mut buf = [0; 16];
        decode(&s[..PARTS[0].0], &mut buf[..PARTS[0].1])?;
        decode(&s[PARTS[0].0 + 1..PARTS[1].0], &mut buf[PARTS[0].1..PARTS[1].1])?;
        decode(&s[PARTS[1].0 + 1..PARTS[2].0], &mut buf[PARTS[1].1..PARTS[2].1])?;
        decode(&s[PARTS[2].0 + 1..PARTS[3].0], &mut buf[PARTS[2].1..PARTS[3].1])?;
        decode(&s[PARTS[3].0 + 1..], &mut buf[PARTS[3].1..])?;
        Ok(buf.into())
    }
}

#[derive(Debug)]
pub struct UuidParseError(());

impl fmt::Display for UuidParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid UUID string")
    }
}

impl std::error::Error for UuidParseError {}

object_ptr_wrapper!(NSUUID);

impl NSUUID {
    pub fn from_uuid(uuid: Uuid) -> StrongPtr<Self> {
        unsafe {
            let mut r: *mut Object = msg_send![class!(NSUUID), alloc];
            r = msg_send![r, initWithUUIDBytes:uuid.as_ptr()];
            StrongPtr::wrap(Self::wrap(r))
        }
    }

    pub fn to_uuid(&self) -> Uuid {
        unsafe {
            let mut r = Uuid::zeroed();
            let _: () = msg_send![self.as_ptr(), getUUIDBytes:r.as_mut_ptr()];
            r
        }
    }
}

object_ptr_wrapper!(CBUUID);

impl CBUUID {
    pub fn from_uuid(uuid: Uuid) -> Self {
        unsafe {
            let data = NSData::from_bytes(uuid.shorten());
            let r: *mut Object = msg_send![class!(CBUUID), UUIDWithData:data];
            Self::wrap(r)
        }
    }

    pub fn array_from_uuids(uuids: &[Uuid]) -> NSArray {
        NSArray::from_iter(uuids.iter().copied().map(CBUUID::from_uuid))
    }

    pub fn to_uuid(&self) -> Uuid {
        let data = unsafe {
            let data: *mut Object = msg_send![self.as_ptr(), data];
            NSData::wrap(data)
        };
        Uuid::from_slice(data.as_bytes())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn shorten() {
        fn base(prefix: &[u8]) -> [u8; 16] {
            let mut r = BASE_UUID_BYTES;
            r[..prefix.len()].copy_from_slice(&prefix);
            r
        }
        let data = &[
            (base(&[0, 0, 0, 0]), &[0, 0][..]),
            (base(&[0, 0, 0, 1]), &[0, 1][..]),
            (base(&[0, 0, 0xff, 0xff]), &[0xff, 0xff][..]),
            (base(&[0, 1, 0, 0]), &[0, 1, 0, 0][..]),
            (base(&[0xff, 0xff, 0xff, 0xff]), &[0xff, 0xff, 0xff, 0xff][..]),
            (base(&[0xff, 0xff, 0xff, 0xff]), &[0xff, 0xff, 0xff, 0xff][..]),
            (base(&[0, 0, 0, 0, 1]), &base(&[0, 0, 0, 0, 1])[..]),
        ];
        for &(inp, exp) in data {
            assert_eq!(Uuid::from_bytes(inp).shorten(), exp);
        }
    }

    #[test]
    fn parse_ok() {
        let data = &[
            ("00000000-0000-0000-0000-000000000000", Uuid::zeroed()),
            ("12345678-9AbC-Def0-1234-56789aBCDEF0", Uuid::from_bytes(
                [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC,
                    0xDE, 0xF0])),
            ("00000000-0000-1000-8000-00805F9B34FB", Uuid::base()),
        ];
        for &(inp, exp) in data {
            let act = inp.parse::<Uuid>().unwrap();
            assert_eq!(act, exp);
            assert_eq!(inp.to_ascii_lowercase(), act.to_string());
        }
    }

    #[test]
    fn parse_fail() {
        let data = &[
            "",
            "0",
            "00000000_0000-0000-0000-000000000000",
            "00000000-0000*0000-0000-000000000000",
            "00000000-0000-0000+0000-000000000000",
            "00000000-0000-0000-0000~000000000000",
            "00000000-0000-00z0-0000-000000000000",
            "00000000-0000-0000-0000-_00000000000",
        ];
        for &inp in data {
            assert!(inp.parse::<Uuid>().is_err());
        }
    }
}