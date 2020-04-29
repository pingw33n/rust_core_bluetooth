use enumflags2::BitFlags;
use std::fmt;

use super::*;
use super::descriptor::Descriptor;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum WriteKind {
    WithResponse = 0,
    WithoutResponse = 1,
}

#[derive(BitFlags, Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
enum Property {
    Broadcast                       = 0x01,
    Read                            = 0x02,
    WriteWithoutResponse            = 0x04,
    Write                           = 0x08,
    Notify                          = 0x10,
    Indicate                        = 0x20,
    AuthenticatedSignedWrites       = 0x40,
    ExtendedProperties              = 0x80,
    NotifyEncryptionRequired        = 0x100,
    IndicateEncryptionRequired      = 0x200
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Properties(BitFlags<Property>);

impl Properties {
    fn from_bits_truncate(bits: u32) -> Self {
        Self(BitFlags::from_bits_truncate(bits))
    }
}

macro_rules! properties {
    ($($(#[$attr:meta])* $f:ident => $e:ident,)*) => {
        impl Properties {
            $(
                $(#[$attr])*
                pub fn $f(&self) -> bool {
                    self.0.contains(Property::$e)
                }
            )*
        }
    };
}

properties!(
    #[doc="Characteristic can broadcast its value using a characteristic configuration descriptor."]
    is_broadcast => Broadcast,

    #[doc="A peripheral can read the characteristic’s value."]
    is_read => Read,

    #[doc="A peripheral can write the characteristic’s value, without a response to indicate that the write succeeded."]
    is_write_without_response => WriteWithoutResponse,

    #[doc="A peripheral can write the characteristic’s value, with a response to indicate that the write succeeded."]
    is_write => Write,

    #[doc="The peripheral permits notifications of the characteristic’s value, without a response from the central to indicate receipt of the notification."]
    is_notify => Notify,

    #[doc="The peripheral permits notifications of the characteristic’s value, with a response from the central to indicate receipt of the notification."]
    is_indicate => Indicate,

    #[doc="The peripheral allows signed writes of the characteristic’s value, without a response to indicate the write succeeded."]
    is_authenticated_signed_writes => AuthenticatedSignedWrites,

    #[doc="The characteristic defines additional properties in the extended properties descriptor."]
    is_extended_properties => ExtendedProperties,

    #[doc="Whether only trusted devices can enable notifications of the characteristic’s value."]
    is_notify_encryption_required => NotifyEncryptionRequired,

    #[doc="Whether only trusted devices can enable indications of the characteristic’s value."]
    is_indicate_encryption_required => IndicateEncryptionRequired,
);

assert_impl_all!(Properties: Send, Sync);

impl fmt::Debug for Properties {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Properties")
            .field(&crate::util::BitFlagsDebug(self.0))
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct Characteristic {
    id: Uuid,
    properties: Properties,
    pub(in crate) characteristic: StrongPtr<CBCharacteristic>,
}

assert_impl_all!(Characteristic: Send, Sync);

impl Characteristic {
    pub(in crate) unsafe fn retain(o: impl ObjectPtr) -> Self {
        let characteristic = CBCharacteristic::wrap(o).retain();
        Self {
            id: characteristic.id(),
            properties: characteristic.properties(),
            characteristic,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn properties(&self) -> &Properties {
        &self.properties
    }
}

object_ptr_wrapper!(CBCharacteristic);

impl CBCharacteristic {
    pub fn id(&self) -> Uuid {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), UUID];
            CBUUID::wrap(r).to_uuid()
        }
    }

    pub fn properties(&self) -> Properties {
        unsafe {
            let r: u32 = msg_send![self.as_ptr(), properties];
            Properties::from_bits_truncate(r)
        }
    }

    pub fn descriptors(&self) -> Option<Vec<Descriptor>> {
        let arr = unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), descriptors];
            NSArray::wrap_nullable(r)?
        };
        Some(arr.iter()
            .map(|v| unsafe { Descriptor::retain(v) })
            .collect())
    }

    pub fn value(&self) -> Option<Vec<u8>> {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), value];
            let r = NSData::wrap_nullable(r)?;
            Some(r.as_bytes().into())
        }
    }
}