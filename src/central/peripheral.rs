use objc::*;
use objc::runtime::Object;
use static_assertions::assert_impl_all;
use std::ptr::NonNull;

use crate::*;
use crate::platform::*;
use crate::uuid::*;

use super::command;
use super::delegate::Delegate;
use super::characteristic::*;
use super::descriptor::*;
use super::service::*;

#[derive(Clone, Copy, Debug)]
pub struct MaxWriteLen {
    pub(in crate) with_response: usize,
    pub(in crate) without_response: usize,
}

assert_impl_all!(MaxWriteLen: Send);

impl MaxWriteLen {
    pub fn with_response(&self) -> usize {
        self.with_response
    }

    pub fn without_response(&self) -> usize {
        self.without_response
    }
}

#[derive(Clone, Debug)]
pub struct Peripheral {
    id: Uuid,
    pub(in crate) peripheral: StrongPtr<CBPeripheral>,
}

assert_impl_all!(Peripheral: Send, Sync);

impl Peripheral {
    pub(in crate) unsafe fn retain(o: impl ObjectPtr) -> Self {
        let peripheral = CBPeripheral::wrap(o).retain();
        Self {
            id: peripheral.id(),
            peripheral,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn discover_services(&self) {
        self.discover_services_with_uuids0(None);
    }

    pub fn discover_services_with_uuids(&self, uuids: &[Uuid]) {
        self.discover_services_with_uuids0(Some(uuids));
    }

    pub fn discover_included_services(&self) {
        self.discover_included_services0(None);
    }

    pub fn discover_included_services_with_uuids(&self, uuids: &[Uuid]) {
        self.discover_included_services0(Some(uuids))
    }

    pub fn discover_characteristics(&self, service: &Service) {
        self.discover_characteristics0(service, None);
    }

    pub fn discover_characteristics_with_uuids(&self, service: &Service, uuids: &[Uuid]) {
        self.discover_characteristics0(service, Some(uuids));
    }

    pub fn discover_descriptors(&self, characteristic: &Characteristic) {
        objc::rc::autoreleasepool(|| {
            self.characteristic_cmd(characteristic)
                .discover_descriptors();
        })
    }

    pub fn subscribe(&self, characteristic: &Characteristic) {
        objc::rc::autoreleasepool(|| {
            self.characteristic_cmd(characteristic)
                .subscribe();
        })
    }

    pub fn unsubscribe(&self, characteristic: &Characteristic) {
        objc::rc::autoreleasepool(|| {
            self.characteristic_cmd(characteristic)
                .unsubscribe();
        })
    }

    pub fn read_characteristic(&self, characteristic: &Characteristic) {
        objc::rc::autoreleasepool(|| {
            self.characteristic_cmd(characteristic)
                .read();
        })
    }

    pub fn write_characteristic(&self, characteristic: &Characteristic, value: &[u8], kind: WriteKind) {
        objc::rc::autoreleasepool(|| {
            command::WriteCharacteristic {
                peripheral: self.peripheral.clone(),
                characteristic: characteristic.characteristic.clone(),
                value: NSData::from_bytes(value).retain(),
                kind,
            }.dispatch();
        })
    }

    pub fn read_descriptor(&self, descriptor: &Descriptor) {
        objc::rc::autoreleasepool(|| {
            command::Descriptor {
                peripheral: self.peripheral.clone(),
                descriptor: descriptor.descriptor.clone(),
            }.read();
        })
    }

    pub fn write_descriptor(&self, descriptor: &Descriptor, value: &[u8]) {
        objc::rc::autoreleasepool(|| {
            command::WriteDescriptor {
                peripheral: self.peripheral.clone(),
                descriptor: descriptor.descriptor.clone(),
                value: NSData::from_bytes(value).retain(),
            }.dispatch();
        })
    }

    pub fn read_rssi(&self) {
        objc::rc::autoreleasepool(|| {
            command::Peripheral {
                peripheral: self.peripheral.clone(),
            }.read_rssi();
        })
    }

    pub fn get_max_write_len(&self) {
        self.get_max_write_len_tagged0(None);
    }

    pub fn get_max_write_len_tagged(&self, tag: Tag) {
        self.get_max_write_len_tagged0(Some(tag));
    }

    fn get_max_write_len_tagged0(&self, tag: Option<Tag>) {
        objc::rc::autoreleasepool(|| {
            command::PeripheralTag {
                peripheral: self.peripheral.clone(),
                tag,
            }.get_max_write_len();
        })
    }

    fn discover_services_with_uuids0(&self, uuids: Option<&[Uuid]>) {
        objc::rc::autoreleasepool(|| {
            let uuids = uuids.map(CBUUID::array_from_uuids).map(|v| v.retain());
            command::DiscoverServices {
                peripheral: self.peripheral.clone(),
                uuids,
            }.dispatch();
        })
    }

    fn discover_included_services0(&self, uuids: Option<&[Uuid]>) {
        objc::rc::autoreleasepool(|| {
            let uuids = uuids.map(CBUUID::array_from_uuids).map(|v| v.retain());
            command::DiscoverServices {
                peripheral: self.peripheral.clone(),
                uuids,
            }.dispatch();
        })
    }

    fn discover_characteristics0(&self, service: &Service, uuids: Option<&[Uuid]>) {
        objc::rc::autoreleasepool(|| {
            let uuids = uuids.map(CBUUID::array_from_uuids).map(|v| v.retain());
            command::DiscoverCharacteristics {
                peripheral: self.peripheral.clone(),
                service: service.service.clone(),
                uuids,
            }.dispatch();
        })
    }

    fn characteristic_cmd(&self, characteristic: &Characteristic) -> command::Characteristic {
        command::Characteristic {
            peripheral: self.peripheral.clone(),
            characteristic: characteristic.characteristic.clone(),
        }
    }
}

impl PartialEq for Peripheral {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Peripheral {}

impl std::hash::Hash for Peripheral {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.id)
    }
}

object_ptr_wrapper!(CBPeripheral);

impl CBPeripheral {
    pub fn id(&self) -> Uuid {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), identifier];
            NSUUID::wrap(r).to_uuid()
        }
    }

    pub fn name(&self) -> Option<NSString> {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), name];
            if r.is_null() {
                None
            } else {
                Some(NSString::wrap(r))
            }
        }
    }

    pub fn set_delegate(&self, delegate: impl ObjectPtr) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), setDelegate:delegate];
        }
    }

    pub fn delegate(&self) -> Delegate {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), delegate];
            Delegate::wrap(NonNull::new(r).unwrap())
        }
    }

    pub fn services(&self) -> Option<Vec<Service>> {
        let arr = unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), services];
            NSArray::wrap_nullable(r)?
        };
        Some(arr.iter()
            .map(|v| unsafe { Service::retain(v) })
            .collect())

    }

    pub fn included_services(&self) -> Option<Vec<Service>> {
        let arr = unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), includedServices];
            NSArray::wrap_nullable(r)?
        };
        Some(arr.iter()
            .map(|v| unsafe { Service::retain(v) })
            .collect())

    }

    pub fn discover_services(&self, uuids: Option<NSArray>) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), discoverServices:uuids.as_ptr()];
        }
    }

    pub fn discover_characteristics(&self, service: CBService, uuids: Option<NSArray>) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), discoverCharacteristics:uuids.as_ptr() forService:service.as_ptr()];
        }
    }

    pub fn discover_descriptors(&self, characteristic: CBCharacteristic) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), discoverDescriptorsForCharacteristic:characteristic.as_ptr()];
        }
    }

    pub fn set_notify_value(&self, characteristic: CBCharacteristic, enabled: bool) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), setNotifyValue:enabled forCharacteristic:characteristic.as_ptr()];
        }
    }

    pub fn read_characteristic(&self, characteristic: CBCharacteristic) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), readValueForCharacteristic:characteristic.as_ptr()];
        }
    }

    pub fn read_descriptor(&self, characteristic: CBDescriptor) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), readValueForDescriptor:characteristic.as_ptr()];
        }
    }

    pub fn write_characteristic(&self, characteristic: CBCharacteristic, value: NSData, kind: WriteKind) {
        unsafe {
            let ty = kind as NSUInteger;
            let _: () = msg_send![self.as_ptr(), writeValue:value forCharacteristic:characteristic.as_ptr() type:ty];
        }
    }

    pub fn write_descriptor(&self, descriptor: CBDescriptor, value: NSData) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), writeValue:value forDescriptor:descriptor.as_ptr()];
        }
    }

    pub fn read_rssi(&self) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), readRSSI];
        }
    }

    pub fn max_write_len(&self, kind: WriteKind) -> usize {
        unsafe {
            let ty = kind as NSUInteger;
            let r: usize = msg_send![self.as_ptr(), maximumWriteValueLengthForType:ty];
            r
        }
    }
}