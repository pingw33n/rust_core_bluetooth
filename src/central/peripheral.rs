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

/// Information about maximum write lengths obtained via
/// [`get_max_write_len`](struct.Peripheral.html#method.get_max_write_len) method.
#[derive(Clone, Copy, Debug)]
pub struct MaxWriteLen {
    pub(in crate) with_response: usize,
    pub(in crate) without_response: usize,
}

assert_impl_all!(MaxWriteLen: Send);

impl MaxWriteLen {
    /// Maximum write length for writes with response.
    pub fn with_response(&self) -> usize {
        self.with_response
    }

    /// Maximum write length for writes without response.
    pub fn without_response(&self) -> usize {
        self.without_response
    }
}

/// A remote peripheral device.
///
/// The `Peripheral` object represents remote peripheral devices that your app discovers with a
/// [central manager](../struct.CentralManager.html). Peripherals use universally unique identifiers
/// (UUIDs) to identify themselves. Peripherals may contain one or more services or provide useful
/// information about their connected signal strength.
///
/// You use this object to discover, explore, and interact with the services available on a remote
/// peripheral that supports Bluetooth low energy. A service encapsulates the way part of the device
/// behaves. For example, one service of a heart rate monitor may be to expose heart rate data from
/// a sensor. Services themselves contain of characteristics or included services (references to
/// other services). Characteristics provide further details about a peripheral’s service.
/// For example, the heart rate service may contain multiple characteristics.
/// One characteristic could describe the intended body location of the device’s heart rate sensor,
/// and another characteristic could transmit the heart rate measurement data. Finally,
/// characteristics contain any number of descriptors that provide more information about the
/// characteristic’s value, such as a human-readable description and a way to format the value.
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

    /// Peripheral identifier.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Discovers all available services of the peripheral.
    ///
    /// See [`discover_services_with_uuids`](struct.Peripheral.html#method.discover_services_with_uuids).
    pub fn discover_services(&self) {
        self.discover_services_with_uuids0(None);
    }

    /// Discovers peripheral's services with the specified `uuids`.
    ///
    /// When the peripheral discovers one or more services, it triggers
    /// [`ServicesDiscovered`](../enum.CentralEvent.html#variant.ServicesDiscovered) event.
    pub fn discover_services_with_uuids(&self, uuids: &[Uuid]) {
        self.discover_services_with_uuids0(Some(uuids));
    }

    /// Discovers all available included services of a previously-discovered service.
    ///
    /// See [`discover_included_services_with_uuids`](struct.Peripheral.html#method.discover_included_services_with_uuids)
    /// method.
    pub fn discover_included_services(&self, service: &Service) {
        self.discover_included_services0(service, None);
    }

    /// Discovers the specified included services of a previously-discovered service.
    ///
    /// When the peripheral discovers one or more included services, it triggers
    /// [`IncludedServicesDiscovered`](../enum.CentralEvent.html#variant.IncludedServicesDiscovered)
    /// event.
    pub fn discover_included_services_with_uuids(&self, service: &Service, uuids: &[Uuid]) {
        self.discover_included_services0(service, Some(uuids))
    }

    /// Discovers all available characteristics of a service.
    ///
    /// See [`discover_characteristics_with_uuids`](struct.Peripheral.html#method.discover_characteristics_with_uuids)
    /// method.
    pub fn discover_characteristics(&self, service: &Service) {
        self.discover_characteristics0(service, None);
    }

    /// Discovers the specified characteristics of a service.
    ///
    /// When the peripheral discovers one or more characteristics, it triggers
    /// [`CharacteristicsDiscovered`](../enum.CentralEvent.html#variant.CharacteristicsDiscovered)
    /// event.
    pub fn discover_characteristics_with_uuids(&self, service: &Service, uuids: &[Uuid]) {
        self.discover_characteristics0(service, Some(uuids));
    }

    /// Discovers the descriptors of a characteristic.
    ///
    /// When the peripheral discovers one or more descriptors, it triggers
    /// [`DescriptorsDiscovered`](../enum.CentralEvent.html#variant.DescriptorsDiscovered)
    /// event.
    pub fn discover_descriptors(&self, characteristic: &Characteristic) {
        objc::rc::autoreleasepool(|| {
            self.characteristic_cmd(characteristic)
                .discover_descriptors();
        })
    }

    /// Subscribes to notifications or indications of the value of a specified characteristic.
    ///
    /// After calling this method the peripheral triggers
    /// [`SubscriptionChangeResult`](../enum.CentralEvent.html#variant.SubscriptionChangeResult)
    /// event to inform whether the action succeeded. If successful, the peripheral then triggers
    /// [`CharacteristicValue`](../enum.CentralEvent.html#variant.CharacteristicValue) event
    /// whenever the characteristic value changes.
    ///
    /// Because the peripheral chooses when it sends an update, your app should prepare to handle
    /// them as long as subscription remains active. If the specified characteristic’s configuration
    /// allows both notifications and indications, calling this method enables notifications only.
    /// You can disable notifications and indications for a characteristic’s value by calling
    /// [`unsubscribe`](struct.Peripheral.html#method.unsubscribe) method.
    pub fn subscribe(&self, characteristic: &Characteristic) {
        objc::rc::autoreleasepool(|| {
            self.characteristic_cmd(characteristic)
                .subscribe();
        })
    }

    /// Cancel subscription for characteristic value created by
    /// [`subscribe`](struct.Peripheral.html#method.subscribe) method.
    pub fn unsubscribe(&self, characteristic: &Characteristic) {
        objc::rc::autoreleasepool(|| {
            self.characteristic_cmd(characteristic)
                .unsubscribe();
        })
    }

    /// Retrieves the value of a specified characteristic.
    ///
    /// After calling this method the peripheral triggers
    /// [`CharacteristicValue`](../enum.CentralEvent.html#variant.CharacteristicValue) event.
    ///
    /// Not all characteristics have a readable value. You can determine whether a characteristic’s
    /// value is readable by accessing the relevant properties of the [`Properties`](../characteristic/struct.Properties.html)
    /// object.
    pub fn read_characteristic(&self, characteristic: &Characteristic) {
        objc::rc::autoreleasepool(|| {
            self.characteristic_cmd(characteristic)
                .read();
        })
    }

    /// Writes the value of a characteristic.
    ///
    /// When you call this method to write the value of a characteristic, the peripheral triggers
    /// [`WriteCharacteristicResult`](../enum.CentralEvent.html#variant.WriteCharacteristicResult)
    /// event, but only if the [`WithResponse`](../characteristic/enum.WriteKind.html#variant.WithResponse)
    /// write kind is requested.
    ///
    /// If you specify the write kind as [`WithoutResponse`](../characteristic/enum.WriteKind.html#variant.WithoutResponse),
    /// Core Bluetooth attempts to write the value but doesn’t guarantee success. If the write
    /// doesn’t succeed in this case, you aren’t notified and you don’t receive an error indicating
    /// the cause of the failure.
    ///
    /// Examine [`can_write`](../characteristic/struct.Properties.html#method.can_write) and
    /// [`can_write_without_response`](../characteristic/struct.Properties.html#method.can_write_without_response)
    /// properties to determine which kinds of writes you can perform.
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

    /// Retrieves the value of a specified characteristic descriptor.
    ///
    /// After calling this method the peripheral triggers
    /// [`DescriptorValue`](../enum.CentralEvent.html#variant.DescriptorValue) event.
    pub fn read_descriptor(&self, descriptor: &Descriptor) {
        objc::rc::autoreleasepool(|| {
            command::Descriptor {
                peripheral: self.peripheral.clone(),
                descriptor: descriptor.descriptor.clone(),
            }.read();
        })
    }

    /// Writes the value of a characteristic descriptor.
    ///
    /// When you call this method to write the value of a characteristic, the peripheral triggers
    /// [`WriteDescriptorResult`](../enum.CentralEvent.html#variant.WriteDescriptorResult) event.
    pub fn write_descriptor(&self, descriptor: &Descriptor, value: &[u8]) {
        objc::rc::autoreleasepool(|| {
            command::WriteDescriptor {
                peripheral: self.peripheral.clone(),
                descriptor: descriptor.descriptor.clone(),
                value: NSData::from_bytes(value).retain(),
            }.dispatch();
        })
    }

    /// Retrieves the current RSSI value for the peripheral while connected to the central manager.
    ///
    /// After calling this method the peripheral triggers
    /// [`ReadRssiResult`](../enum.CentralEvent.html#variant.ReadRssiResult) event.
    pub fn read_rssi(&self) {
        objc::rc::autoreleasepool(|| {
            command::Peripheral {
                peripheral: self.peripheral.clone(),
            }.read_rssi();
        })
    }

    /// Queries for maximum length of data that can be written to characteristic in a single
    /// request. The result is returned as
    /// [`GetMaxWriteLenResult`](../enum.CentralEvent.html#variant.GetMaxWriteLenResult) event.
    pub fn get_max_write_len(&self) {
        self.get_max_write_len_tagged0(None);
    }

    /// Allows tagging an asynchronous [`get_max_write_len`](struct.Peripheral.html#method.get_max_write_len)
    /// call with arbitrary `tag`.
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

    fn discover_included_services0(&self, service: &Service, uuids: Option<&[Uuid]>) {
        objc::rc::autoreleasepool(|| {
            let uuids = uuids.map(CBUUID::array_from_uuids).map(|v| v.retain());
            command::PeripheralServiceUuids {
                peripheral: self.peripheral.clone(),
                service: service.service.clone(),
                uuids,
            }.discover_included_services();
        })
    }

    fn discover_characteristics0(&self, service: &Service, uuids: Option<&[Uuid]>) {
        objc::rc::autoreleasepool(|| {
            let uuids = uuids.map(CBUUID::array_from_uuids).map(|v| v.retain());
            command::PeripheralServiceUuids {
                peripheral: self.peripheral.clone(),
                service: service.service.clone(),
                uuids,
            }.discover_characteristics();
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

    pub fn discover_included_services(&self, service: CBService, uuids: Option<NSArray>) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), discoverIncludedServices:uuids.as_ptr() forService:service.as_ptr()];
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