mod command;
mod delegate;
pub mod characteristic;
pub mod descriptor;
pub mod peripheral;
pub mod service;

use objc::*;
use objc::runtime::*;
use static_assertions::*;
use std::os::raw::*;
use std::sync::Arc;
use std::mem;
use std::ptr;
use std::ptr::NonNull;
use std::collections::HashMap;

use crate::*;
use crate::error::Error;
use crate::platform::*;
use crate::sync;
use crate::uuid::*;

use characteristic::Characteristic;
use delegate::Delegate;
use descriptor::Descriptor;
use peripheral::*;
use service::Service;

#[derive(Debug)]
#[non_exhaustive]
pub enum CentralEvent {
    CharacteristicsDiscovered {
        peripheral: Peripheral,
        service: Service,
        characteristics: Result<Vec<Characteristic>, Error>,
    },

    CharacteristicValue {
        peripheral: Peripheral,
        characteristic: Characteristic,
        value: Result<Vec<u8>, Error>,
    },

    DescriptorsDiscovered {
        peripheral: Peripheral,
        characteristic: Characteristic,
        descriptors: Result<Vec<Descriptor>, Error>,
    },

    DescriptorValue {
        peripheral: Peripheral,
        descriptor: Descriptor,
        value: Result<Vec<u8>, Error>,
    },

    GetMaxWriteLenResult {
        max_write_len: MaxWriteLen,
        tag: Option<Tag>,
    },

    GetPeripheralsResult {
        peripherals: Vec<Peripheral>,
        tag: Option<Tag>,
    },

    GetPeripheralsWithServicesResult {
        peripherals: Vec<Peripheral>,
        tag: Option<Tag>,
    },

    IncludedServicesDiscovered {
        peripheral: Peripheral,
        service: Service,
        included_services: Result<Vec<Service>, Error>,
    },

    ManagerStateChanged {
        new_state: ManagerState,
    },

    PeripheralConnected {
        peripheral: Peripheral,
    },

    PeripheralConnectFailed {
        peripheral: Peripheral,
        error: Option<Error>,
    },

    PeripheralDisconnected {
        peripheral: Peripheral,
        error: Option<Error>,
    },

    PeripheralDiscovered {
        peripheral: Peripheral,
        advertisement_data: AdvertisementData,
        rssi: i32,
    },

    PeripheralIsReadyToWriteWithoutResponse {
        peripheral: Peripheral,
    },

    PeripheralNameChanged {
        peripheral: Peripheral,
        new_name: Option<String>,
    },

    ReadRssiResult {
        peripheral: Peripheral,
        rssi: Result<i32, Error>,
    },

    ServicesChanged {
        peripheral: Peripheral,
        services: Vec<Service>,
        invalidated_services: Vec<Service>,
    },

    ServicesDiscovered {
        peripheral: Peripheral,
        services: Result<Vec<Service>, Error>,
    },

    SubscriptionChanged {
        peripheral: Peripheral,
        characteristic: Characteristic,
        result: Result<(), Error>,
    },

    WriteCharacteristicResult {
        peripheral: Peripheral,
        characteristic: Characteristic,
        result: Result<(), Error>,
    },

    WriteDescriptorResult {
        peripheral: Peripheral,
        descriptor: Descriptor,
        result: Result<(), Error>,
    },
}

assert_impl_all!(CentralEvent: Send);
assert_not_impl_any!(CentralEvent: Sync);

pub struct CentralManagerBuilder {
    show_power_alert: bool,
}

impl CentralManagerBuilder {
    pub fn new() -> Self {
        Self {
            show_power_alert: false,
        }
    }

    pub fn show_power_alert_alert(&mut self, v: bool) -> &mut Self {
        self.show_power_alert = v;
        self
    }

    pub fn build(&self) -> (CentralManager, sync::Receiver<CentralEvent>) {
        CentralManager::build(self)
    }
}

assert_impl_all!(CentralManagerBuilder: Send, Sync);

#[derive(Default)]
pub struct ScanOptions {
    allow_duplicates: bool,
    service_cbuuids: Option<StrongPtr<NSArray>>,
    solicited_service_cbuuids: Option<StrongPtr<NSArray>>,
}

impl ScanOptions {
    pub fn allow_duplicates(mut self, v: bool) -> Self {
        self.allow_duplicates = v;
        self
    }

    pub fn services(mut self, uuids: &[Uuid]) -> Self {
        if self.service_cbuuids.is_none() {
            self.service_cbuuids = Some(NSArray::with_capacity(uuids.len()).retain());
        }
        for &uuid in uuids {
            self.service_cbuuids.as_ref().unwrap().push(CBUUID::from_uuid(uuid));
        }
        self
    }

    pub fn solicited_services(mut self, uuids: &[Uuid]) -> Self {
        if self.solicited_service_cbuuids.is_none() {
            self.solicited_service_cbuuids = Some(NSArray::with_capacity(uuids.len()).retain());
        }
        for &uuid in uuids {
            self.solicited_service_cbuuids.as_ref().unwrap().push(CBUUID::from_uuid(uuid));
        }
        self
    }

    fn to_options_dict(&self) -> NSDictionary {
        let dict = NSDictionary::with_capacity(2);
        dict.insert(unsafe { CBCentralManagerScanOptionAllowDuplicatesKey }, NSNumber::new_bool(self.allow_duplicates));
        if let Some(uuids) = self.solicited_service_cbuuids.as_ref() {
            dict.insert(unsafe { CBCentralManagerScanOptionSolicitedServiceUUIDsKey }, uuids.as_ptr());
        }
        dict
    }
}

assert_impl_all!(ScanOptions: Send, Sync);

#[derive(Default)]
pub struct ConnectOptions {
    notify_on_connection: bool,
    notify_on_disconnection: bool,
    notify_on_notification: bool,
    start_delay_seconds: u32,
}

impl ConnectOptions {
    fn to_options_dict(&self) -> NSDictionary {
        let dict = NSDictionary::with_capacity(6);
        dict.insert(unsafe { CBConnectPeripheralOptionNotifyOnConnectionKey },
            NSNumber::new_bool(self.notify_on_connection));
        dict.insert(unsafe { CBConnectPeripheralOptionNotifyOnDisconnectionKey },
            NSNumber::new_bool(self.notify_on_disconnection));
        dict.insert(unsafe { CBConnectPeripheralOptionNotifyOnNotificationKey },
            NSNumber::new_bool(self.notify_on_notification));
        dict.insert(unsafe { CBConnectPeripheralOptionStartDelayKey },
            NSNumber::new_u32(self.start_delay_seconds));
        dict
    }
}

assert_impl_all!(ConnectOptions: Send, Sync);

struct Inner {
    manager: StrongPtr<CBCentralManager>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        command::Manager {
            manager: self.manager.clone(),
        }.drop_self();
    }
}

#[derive(Clone)]
// TODO The only reason why Arc is needed here is that we need to cleanup Delegate resources.
// Unfortunately objc lib can't replace methods.
pub struct CentralManager(Arc<Inner>);

assert_impl_all!(CentralManager: Send, Sync);

impl CentralManager {
    pub fn new() -> (Self, sync::Receiver<CentralEvent>) {
        objc::rc::autoreleasepool(|| {
            CentralManagerBuilder::new().build()
        })
    }

    pub fn get_peripherals(&self, uuids: &[Uuid]) {
        self.get_peripherals_tagged0(uuids, None);
    }

    pub fn get_peripherals_tagged(&self, uuids: &[Uuid], tag: Tag) {
        self.get_peripherals_tagged0(uuids, Some(tag))
    }

    pub fn get_peripherals_with_services(&self, services_uuids: &[Uuid]) {
        self.get_peripherals_with_services_tagged0(services_uuids, None);
    }

    pub fn get_peripherals_with_services_tagged(&self, services_uuids: &[Uuid], tag: Tag) {
        self.get_peripherals_with_services_tagged0(services_uuids, Some(tag));
    }

    pub fn scan(&self) {
        self.scan_with_options(Default::default());
    }

    pub fn scan_with_options(&self, options: ScanOptions) {
        objc::rc::autoreleasepool(|| {
            command::Scan {
                manager: self.0.manager.clone(),
                options,
            }.dispatch()
        })
    }

    /// Asks the central manager to stop scanning for peripherals.
    pub fn cancel_scan(&self) {
        objc::rc::autoreleasepool(|| {
            command::Manager {
                manager: self.0.manager.clone(),
            }.cancel_scan();
        })
    }

    pub fn connect(&self, peripheral: &Peripheral) {
        self.connect_with_options(peripheral, Default::default());
    }

    pub fn connect_with_options(&self, peripheral: &Peripheral, options: ConnectOptions) {
        objc::rc::autoreleasepool(|| {
            command::Connect {
                manager: self.0.manager.clone(),
                peripheral: peripheral.peripheral.clone(),
                options,
            }.dispatch()
        })
    }

    /// Cancels an active or pending local connection to a peripheral.
    pub fn cancel_connect(&self, peripheral: &Peripheral) {
        objc::rc::autoreleasepool(|| {
            command::CancelConnect {
                manager: self.0.manager.clone(),
                peripheral: peripheral.peripheral.clone(),
            }.cancel_connect()
        })
    }

    fn get_peripherals_tagged0(&self, uuids: &[Uuid], tag: Option<Tag>) {
        objc::rc::autoreleasepool(|| {
            let uuids = NSArray::from_iter(uuids.iter().copied().map(NSUUID::from_uuid)).retain();
            command::GetPeripherals {
                manager: self.0.manager.clone(),
                uuids,
                tag,
            }.get_peripherals()
        })
    }

    fn get_peripherals_with_services_tagged0(&self, services_uuids: &[Uuid], tag: Option<Tag>) {
        objc::rc::autoreleasepool(|| {
            let uuids = CBUUID::array_from_uuids(services_uuids).retain();
            command::GetPeripherals {
                manager: self.0.manager.clone(),
                uuids,
                tag,
            }.get_peripherals_with_services()
        })
    }

    fn build(b: &CentralManagerBuilder) -> (Self, sync::Receiver<CentralEvent>) {
        let (manager, recv) = CBCentralManager::new(b.show_power_alert);
        (Self(Arc::new(Inner {
            manager,
        })), recv)
    }
}

object_ptr_wrapper!(CBCentralManager);

impl CBCentralManager {
    pub fn new(show_power_alert: bool) -> (StrongPtr<Self>, sync::Receiver<CentralEvent>) {
        let (sender, receiver) = sync::channel();

        unsafe {
            let queue = dispatch_queue_create(ptr::null(), DISPATCH_QUEUE_SERIAL);

            let delegate = Delegate::new(sender, queue);

            let options = NSDictionary::with_capacity(1);
            options.insert(CBCentralManagerOptionShowPowerAlertKey, NSNumber::new_bool(show_power_alert));

            let mut r: *mut Object = msg_send![class!(CBCentralManager), alloc];
            r = msg_send![r.as_ptr(), initWithDelegate:delegate queue:queue options:options];
            let r = StrongPtr::wrap(Self::wrap(r));

            (r, receiver)
        }
    }

    fn drop_self(&self) {
        self.delegate().drop_self();
    }

    fn delegate(&self) -> Delegate {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), delegate];
            Delegate::wrap(NonNull::new(r).unwrap())
        }
    }

    fn state(&self) -> ManagerState {
        unsafe {
            let r: c_int = msg_send![self.as_ptr(), state];
            ManagerState::from_u8(r as u8)
                .unwrap_or(ManagerState::Unknown)
        }
    }

    fn scan(&self, options: &ScanOptions) {
        let services = options.service_cbuuids.as_ptr();
        let options = options.to_options_dict();
        unsafe {
            let _: () = msg_send![self.as_ptr(), scanForPeripheralsWithServices:services options:options];
        }
    }

    fn cancel_scan(&self) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), stopScan];
        }
    }

    fn connect(&self, peripheral: &CBPeripheral, options: &ConnectOptions) {
        let options = options.to_options_dict();
        unsafe {
            let _: () = msg_send![self.as_ptr(), connectPeripheral:peripheral.as_ptr() options:options];
        }
    }

    fn cancel_connect(&self, peripheral: &CBPeripheral) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), cancelPeripheralConnection:peripheral.as_ptr()];
        }
    }

    fn get_peripherals(&self, uuids: NSArray /* of NSUUID */) -> Option<Vec<Peripheral>> {
        let r = unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), retrievePeripheralsWithIdentifiers:uuids.as_ptr()];
            NSArray::wrap_nullable(r)
        };
        r.map(|r| r.iter()
            .map(|v| unsafe { Peripheral::retain(v) })
            .inspect(|v| v.peripheral.set_delegate(self.delegate()))
            .collect())
    }

    fn get_peripherals_with_services(&self, uuids: NSArray /* of CBUUID */) -> Option<Vec<Peripheral>> {
        let r = unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), retrieveConnectedPeripheralsWithServices:uuids.as_ptr()];
            NSArray::wrap_nullable(r)
        };
        r.map(|r| r.iter()
            .map(|v| unsafe { Peripheral::retain(v) })
            .inspect(|v| v.peripheral.set_delegate(self.delegate()))
            .collect())
    }
}

#[derive(Clone, Debug)]
pub struct AdvertisementData {
    connectable: Option<bool>,
    local_name: Option<String>,
    manufacturer_data: Option<Vec<u8>>,
    service_data: ServiceData,
    service_uuids: Vec<Uuid>,
    solicited_service_uuids: Vec<Uuid>,
    overflow_service_uuids: Vec<Uuid>,
    // TODO what is the type exactly?
    tx_power_level: Option<i32>,
}

assert_impl_all!(AdvertisementData: Send, Sync);

impl AdvertisementData {
    pub(in crate) fn from_dict(dict: NSDictionary) -> Self {
        let connectable = dict.get(unsafe { CBAdvertisementDataIsConnectable })
            .map(|r| unsafe { NSNumber::wrap(r) }.get_bool() );
        let local_name = dict.get(unsafe { CBAdvertisementDataLocalNameKey })
            .map(|r| unsafe { NSString::wrap(r) }.as_str().to_owned() );
        let manufacturer_data = dict.get(unsafe { CBAdvertisementDataManufacturerDataKey })
            .map(|r| { unsafe { NSData::wrap(r) }; unsafe { NSData::wrap(r) }.as_bytes().to_owned() });
        let service_data = dict.get(unsafe { CBAdvertisementDataServiceDataKey })
            .map(|r| ServiceData::from_dict(unsafe { NSDictionary::wrap(r) } ))
            .unwrap_or(ServiceData::new());
        let get_uuids = |key| {
            dict.get(key)
                .map(|r| {
                    unsafe { NSArray::wrap(r) }
                        .iter()
                        .map(|obj| unsafe { CBUUID::wrap(obj) }.to_uuid())
                        .collect::<Vec<_>>()
                })
                .unwrap_or(Vec::new())
        };
        let service_uuids = get_uuids(unsafe { CBAdvertisementDataServiceUUIDsKey });
        let overflow_service_uuids = get_uuids(unsafe { CBAdvertisementDataOverflowServiceUUIDsKey });
        let solicited_service_uuids = get_uuids(unsafe { CBAdvertisementDataSolicitedServiceUUIDsKey });
        let tx_power_level = dict.get(unsafe { CBAdvertisementDataTxPowerLevelKey })
            .map(|r| unsafe { NSNumber::wrap(r) }.get_i32() );
        Self {
            connectable,
            local_name,
            manufacturer_data,
            service_data,
            service_uuids,
            overflow_service_uuids,
            solicited_service_uuids,
            tx_power_level,
        }
    }

    /// Indicates whether the advertising event type is connectable.
    /// You can use this value to determine whether your app can currently connect to a peripheral.
    pub fn is_connectable(&self) -> Option<bool> {
        self.connectable
    }

    /// The local name of a peripheral.
    pub fn local_name(&self) -> Option<&str> {
        self.local_name.as_ref().map(|v| v.as_str())
    }

    /// The manufacturer data of a peripheral.
    pub fn manufacturer_data(&self) -> Option<&[u8]> {
        self.manufacturer_data.as_ref().map(|v| v.as_slice())
    }

    /// Service-specific advertisement data.
    pub fn service_data(&self) -> &ServiceData {
        &self.service_data
    }

    /// Service UUIDs.
    pub fn service_uuids(&self) -> &[Uuid] {
        &self.service_uuids
    }

    /// Service UUIDs found in the overflow area of the advertisement data.
    pub fn overflow_service_uuids(&self) -> &[Uuid] {
        &self.overflow_service_uuids
    }

    /// Solicited service UUIDs.
    pub fn solicited_service_uuids(&self) -> &[Uuid] {
        &self.solicited_service_uuids
    }

    /// The transmit power of a peripheral.
    /// You can calculate the path loss by comparing the RSSI value with the transmitting power level.
    pub fn tx_power_level(&self) -> Option<i32> {
        self.tx_power_level
    }
}

/// Service-specific advertisement data. The keys represent Service UUIDs.
#[derive(Clone, Debug)]
pub struct ServiceData(HashMap<Uuid, Vec<u8>>);

assert_impl_all!(ServiceData: Send, Sync);

impl ServiceData {
    pub(in crate) fn new() -> Self {
        Self(Default::default())
    }

    pub(in crate) fn from_dict(dict: NSDictionary) -> Self {
        Self(dict.iter()
            .map(|(k, v)| (
                unsafe { CBUUID::wrap(k) }.to_uuid(),
                unsafe { NSData::wrap(v) }.as_bytes().to_owned()))
            .collect())
    }

    pub fn get(&self, uuid: Uuid) -> Option<&[u8]> {
        self.0.get(&uuid).map(|v| v.as_slice())
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item=Uuid> + 'a {
        self.0.keys().copied()
    }

    pub fn values<'a>(&'a self) -> impl Iterator<Item=&[u8]> + 'a {
        self.0.values().map(|v| v.as_slice())
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item=(Uuid, &[u8])> + 'a {
        self.0.iter().map(|(k, v)| (*k, v.as_slice()))
    }
}


