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

/// Events that a central manager sends about changes in its state or state of its local or remote
/// components.
#[derive(Debug)]
#[non_exhaustive]
pub enum CentralEvent {
    /// Indicates the peripheral discovered characteristics for a service.
    ///
    /// This event is triggered in response to the
    /// [`discover_characteristics`](peripheral/struct.Peripheral.html#method.discover_characteristics)
    /// method call.
    CharacteristicsDiscovered {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The service to which the characteristics belong.
        service: Service,

        /// The discovered characteristics or error if the call failed.
        characteristics: Result<Vec<Characteristic>, Error>,
    },

    /// Indicates that retrieving the specified characteristic’s value completed, or that the
    /// characteristic’s value changed.
    ///
    /// This event is triggered in response to the
    /// [`read_characteristic`](peripheral/struct.Peripheral.html#method.read_characteristic)
    /// method call. A peripheral also invokes this method to notify about a change to the
    /// value of the characteristic for which notifications were previously enabled by calling
    /// [`subscribe`](peripheral/struct.Peripheral.html#method.subscribe) method.
    CharacteristicValue {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The characteristic containing the value.
        characteristic: Characteristic,

        /// The value or error if the call failed.
        value: Result<Vec<u8>, Error>,
    },

    /// Indicates the peripheral discovered descriptors for a characteristic.
    ///
    /// This event is triggered in response to the
    /// [`discover_descriptors`](peripheral/struct.Peripheral.html#method.discover_descriptors)
    /// method call.
    DescriptorsDiscovered {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The characteristic to which the descriptors belong.
        characteristic: Characteristic,

        /// The discovered descriptors or error if the call failed.
        descriptors: Result<Vec<Descriptor>, Error>,
    },

    /// Indicates that retrieving the specified characteristic descriptor’s value completed.
    ///
    /// This event is triggered in response to the
    /// [`read_descriptor`](peripheral/struct.Peripheral.html#read_descriptor)
    /// method call.
    DescriptorValue {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The descriptor containing the value.
        descriptor: Descriptor,

        /// The value or error if the call failed.
        value: Result<Vec<u8>, Error>,
    },

    /// Indicates that the [`get_max_write_len`](peripheral/struct.Peripheral.html#method.get_max_write_len)
    /// method call completed.
    GetMaxWriteLenResult {
        /// Maximum write length information.
        max_write_len: MaxWriteLen,

        /// Optional tag specified by [`get_max_write_len_tagged`](peripheral/struct.Peripheral.html#method.get_max_write_len_tagged).
        tag: Option<Tag>,
    },

    /// Indicates that the [`get_peripherals`](struct.CentralManager.html#method.get_peripherals)
    /// method call completed.
    GetPeripheralsResult {
        /// A list of peripherals that the central manager is able to match to the provided identifiers.
        peripherals: Vec<Peripheral>,

        /// Optional tag specified by [`get_peripherals_tagged`](struct.CentralManager.html#method.get_peripherals_tagged).
        tag: Option<Tag>,
    },

    /// Indicates that the [`get_peripherals_with_services`](struct.CentralManager.html#method.get_peripherals_with_services)
    /// method call completed.
    GetPeripheralsWithServicesResult {
        /// A list of the peripherals that are currently connected to the system and that contain
        /// any of the services specified in the `service_uuids` parameter. This list can include
        /// peripherals connected by other apps. You need to [connect](struct.CentralManager.html#method.connect)
        /// them before use.
        peripherals: Vec<Peripheral>,

        /// Optional tag specified by [`get_peripherals_with_services_tagged`](struct.CentralManager.html#method.get_peripherals_with_services_tagged).
        tag: Option<Tag>,
    },

    /// Indicates that discovery of included services within the provided service completed.
    IncludedServicesDiscovered {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The service containing the discovered included services.
        service: Service,

        /// The discovered included services or error if the call failed.
        ///
        /// This event is triggered in response to the
        /// [`discover_included_services`](peripheral/struct.Peripheral.html#method.discover_included_services)
        /// method call.
        included_services: Result<Vec<Service>, Error>,
    },

    /// Indicates the central manager’s state updated.
    ///
    /// You handle this event to ensure that the central device supports Bluetooth low energy and
    /// that it’s available to use. You should issue commands to the central manager only when the
    /// central manager’s state indicates it’s powered on. A state with a value lower than
    /// [PoweredOn](../enum.ManagerState.html#variant.PoweredOn) implies that scanning has stopped,
    /// which in turn disconnects any previously-connected peripherals. If the state moves below
    /// [PoweredOff](../enum.ManagerState.html#variant.PoweredOff), all
    /// [`Peripheral`](peripheral/struct.Peripheral.html) objects obtained from this central manager
    /// become invalid; you must retrieve or discover these peripherals again.
    /// For a complete list of possible states, see the [ManagerState](../enum.ManagerState.html) enum.
    ManagerStateChanged {
        /// Current state of the central manager.
        new_state: ManagerState,
    },

    /// Indicates the central manager connected to the peripheral.
    ///
    /// This event is triggered when a call to [`connect`](struct.CentralManager.html#method.connect)
    /// method succeeds.
    PeripheralConnected {
        /// The now-connected peripheral.
        peripheral: Peripheral,
    },

    /// Indicates the central manager failed to create a connection with the peripheral.
    ///
    /// This event is triggered when connection initiated with the
    /// [`connect`](struct.CentralManager.html#method.connect) method fails to complete. Because connection
    /// attempts don’t time out, a failed connection usually indicates a transient issue, in which
    /// case you may attempt connecting to the peripheral again.
    PeripheralConnectFailed {
        /// The peripheral that failed to connect.
        peripheral: Peripheral,

        /// The cause of the failure, or `None` if no error occurred.
        error: Option<Error>,
    },

    /// Indicates the central manager disconnected from a peripheral.
    ///
    /// This event is triggered when disconnecting a peripheral previously connected with the
    /// [`connect`](struct.CentralManager.html#method.connect) method. The `error` contains the reason for
    /// the disconnection, unless the disconnect resulted from a call to
    /// [`disconnect`](struct.CentralManager.html#method.disconnect). After this event, no other event will be
    /// received for the peripheral.
    ///
    /// All services, characteristics, and characteristic descriptors of the peripheral become
    /// invalidated after it disconnects.
    PeripheralDisconnected {
        /// The now-disconnected peripheral.
        peripheral: Peripheral,

        /// The cause of the failure, or `None` if no error occurred.
        error: Option<Error>,
    },

    /// Indicates the central manager discovered a peripheral while scanning for devices.
    PeripheralDiscovered {
        /// The discovered peripheral.
        peripheral: Peripheral,

        /// Peripheral's advertisment data.
        advertisement_data: AdvertisementData,

        /// The current received signal strength indicator (RSSI) of the peripheral, in decibels.
        ///
        /// Use the RSSI data to determine the proximity of a discoverable peripheral device, and
        /// whether you want to connect to it automatically.
        rssi: i32,
    },

    /// Indicates that a peripheral is again ready to send characteristic updates.
    ///
    /// This event is triggered after a failed call to
    /// [`write_characteristic`](peripheral/struct.Peripheral.html#method.write_characteristic),
    /// once peripheral is ready to send characteristic value updates.
    PeripheralIsReadyToWriteWithoutResponse {
        /// The peripheral providing this update.
        peripheral: Peripheral,
    },

    /// Indicates that a peripheral’s name changed.
    ///
    /// This event is triggered whenever the peripheral’s Generic Access Profile (GAP) device name
    /// changes. Since a peripheral device can change its GAP device name, you can handle this event
    /// if you need to display the current name of the peripheral device.
    PeripheralNameChanged {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The peripheral's name.
        new_name: Option<String>,
    },

    /// Indicates that retrieving the value of the peripheral’s current Received Signal Strength
    /// Indicator (RSSI) completed.
    ///
    /// This event is triggered in response to
    /// [`read_rssi`](peripheral/struct.Peripheral.html#method.read_rssi),
    /// method call.
    ReadRssiResult {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The RSSI, in decibels, or error if the call failed.
        rssi: Result<i32, Error>,
    },

    /// Indicates that a peripheral’s services changed.
    ///
    /// This event is triggered whenever one or more services of a peripheral change. A peripheral’s
    /// services have changed if:
    ///
    /// * The peripheral removes a service from its database.
    /// * The peripheral adds a new service to its database.
    /// * The peripheral adds back a previously-removed service, but at a different location in the
    ///   database.
    ///
    /// The `invalidated_services` includes any changed services that you previously discovered;
    /// you can no longer use these services. You can use the
    /// [`discover_services`](peripheral/struct.Peripheral.html#method.discover_services) method to
    /// discover any new services that the peripheral added to its database.
    /// Use this same method to find out whether any of the invalidated services that you were using
    /// (and want to continue using) now have a different location in the peripheral’s database.
    ServicesChanged {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// A list of services after the change. Note, this doesn't include newly added services.
        services: Vec<Service>,

        /// A list of services invalidated by this change.
        invalidated_services: Vec<Service>,
    },

    /// Peripheral service discovery succeeded.
    ///
    /// This event is triggered in response to the
    /// [`discover_services`](peripheral/struct.Peripheral.html#method.discover_services)
    /// method call.
    ServicesDiscovered {
        /// The peripheral to which the `services` belong.
        peripheral: Peripheral,

        /// The discovered services or error if the call failed.
        services: Result<Vec<Service>, Error>,
    },

    /// Indicates the peripheral received a request to start or stop providing notifications for a
    /// specified characteristic’s value.
    ///
    /// This event is triggered in response to the
    /// [`subscribe`](peripheral/struct.Peripheral.html#method.subscribe) or
    /// [`unsubscribe`](peripheral/struct.Peripheral.html#method.unsubscribe)
    /// methods call.
    SubscriptionChangeResult {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The characteristic for which to configure value notifications.
        characteristic: Characteristic,

        /// Whether the subscription change succeeded.
        result: Result<(), Error>,
    },

    /// Characteristic value write completed.
    ///
    /// This event is triggered in response to the
    /// [`write_characteristic`](peripheral/struct.Peripheral.html#method.write_characteristic)
    /// method called with [`WithResponse`](characteristic/enum.WriteKind.html#variant.WithResponse)
    /// as the `kind` parameter.
    WriteCharacteristicResult {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The target characteristic.
        characteristic: Characteristic,

        /// Whether the write succeeded.
        result: Result<(), Error>,
    },

    /// Characteristic descriptor's value write completed.
    ///
    /// This event is triggered in response to the
    /// [`write_descriptor`](peripheral/struct.Peripheral.html#method.write_descriptor)
    /// method call.
    WriteDescriptorResult {
        /// The peripheral providing this information.
        peripheral: Peripheral,

        /// The target descriptor.
        descriptor: Descriptor,

        /// Whether the write succeeded.
        result: Result<(), Error>,
    },
}

assert_impl_all!(CentralEvent: Send);
assert_not_impl_any!(CentralEvent: Sync);

/// Peripheral scanning options accepted by [`scan_with_options`](struct.CentralManager.html#method.scan_with_options).
#[derive(Default)]
pub struct ScanOptions {
    allow_duplicates: bool,
    service_cbuuids: Option<StrongPtr<NSArray>>,
    solicited_service_cbuuids: Option<StrongPtr<NSArray>>,
}

impl ScanOptions {
    /// Specifies whether the scan should run without duplicate filtering.
    ///
    /// If `true`, the central disables filtering and generates a discovery event each time it
    /// receives an advertising packet from the peripheral. If `false` (the default), the central
    /// coalesces multiple discoveries of the same peripheral into a single discovery event.
    ///
    /// Disabling this filtering can have an adverse effect on battery life; use it only if necessary.
    pub fn allow_duplicates(mut self, v: bool) -> Self {
        self.allow_duplicates = v;
        self
    }

    /// Specifies services UUIDs making the central manager return only peripherals that advertise
    /// these services.
    pub fn include_services(mut self, uuids: &[Uuid]) -> Self {
        if self.service_cbuuids.is_none() {
            self.service_cbuuids = Some(NSArray::with_capacity(uuids.len()).retain());
        }
        for &uuid in uuids {
            self.service_cbuuids.as_ref().unwrap().push(CBUUID::from_uuid(uuid));
        }
        self
    }

    /// Specifying this scan option causes the central manager to also scan for peripherals
    /// soliciting any of the services contained in the array.
    pub fn include_solicited_services(mut self, uuids: &[Uuid]) -> Self {
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

/// An object that scans for, discovers, connects to, and manages peripherals.
///
/// Before calling the `CentralManager` methods,
/// [`ManagerStateChanged`](enum.CentralEvent.html#variant.ManagerStateChanged)
/// event must be received indicating the [PoweredOn](../enum.ManagerState.html#variant.PoweredOn)
/// state.
#[derive(Clone)]
// TODO The only reason why Arc is needed here is that we need to cleanup Delegate resources.
// Unfortunately objc lib can't replace methods.
pub struct CentralManager(Arc<Inner>);

assert_impl_all!(CentralManager: Send, Sync);

impl CentralManager {
    pub fn new() -> (Self, sync::Receiver<CentralEvent>) {
        objc::rc::autoreleasepool(|| {
            let (manager, recv) = CBCentralManager::new(false);
            (Self(Arc::new(Inner {
                manager,
            })), recv)
        })
    }

    /// Returns a list of known peripherals by their identifiers. The result is returned as
    /// [`GetPeripheralsWithServicesResult`](enum.CentralEvent.html#variant.GetPeripheralsWithServicesResult).
    pub fn get_peripherals(&self, uuids: &[Uuid]) {
        self.get_peripherals_tagged0(uuids, None);
    }

    /// Allows tagging an asynchronous [`get_peripherals`](struct.CentralManager.html#method.get_peripherals)
    /// call with arbitrary `tag`.
    pub fn get_peripherals_tagged(&self, uuids: &[Uuid], tag: Tag) {
        self.get_peripherals_tagged0(uuids, Some(tag))
    }

    /// Retrieves a list of the peripherals connected to the system whose services match
    /// the specified `services_uuids`. The result is returned as
    /// [`GetPeripheralsWithServicesResult`](enum.CentralEvent.html#variant.GetPeripheralsWithServicesResult).
    pub fn get_peripherals_with_services(&self, services_uuids: &[Uuid]) {
        self.get_peripherals_with_services_tagged0(services_uuids, None);
    }

    /// Allows tagging an asynchronous [`get_peripherals_with_services`](struct.CentralManager.html#method.get_peripherals_with_services)
    /// call with arbitrary `tag`.
    pub fn get_peripherals_with_services_tagged(&self, services_uuids: &[Uuid], tag: Tag) {
        self.get_peripherals_with_services_tagged0(services_uuids, Some(tag));
    }

    /// Scans for peripherals with default options.
    /// See [`scan_with_options`](struct.CentralManager.html#method.scan_with_options).
    pub fn scan(&self) {
        self.scan_with_options(Default::default());
    }

    /// Scans for peripherals that are advertising services with the specified `options`.
    ///
    /// If the central manager is actively scanning with one set of parameters and it receives
    /// another set to scan, the new parameters override the previous set. When the central manager
    /// discovers a peripheral, it triggers
    /// [`PeripheralDiscovered`](enum.CentralEvent.html#variant.PeripheralDiscovered) event.
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

    /// Establishes a local connection to the `peripheral`.
    ///
    /// After successfully establishing a local connection to a peripheral, the central manager
    /// object triggers [`PeripheralConnected`](enum.CentralEvent.html#variant.PeripheralConnected)
    /// event. If the connection attempt fails, the central manager object calls the
    /// [`PeripheralConnectFailed`](enum.CentralEvent.html#variant.PeripheralConnectFailed) instead.
    /// Attempts to connect to a peripheral don’t time out. To explicitly cancel a pending
    /// connection to a peripheral, call the
    /// [`cancel_connect`](struct.CentralManager.html#method.cancel_connect) method.
    /// Dropping the `Peripheral` also implicitly cancels connection.
    pub fn connect(&self, peripheral: &Peripheral) {
        objc::rc::autoreleasepool(|| {
            command::Connect {
                manager: self.0.manager.clone(),
                peripheral: peripheral.peripheral.clone(),
            }.dispatch()
        })
    }

    /// Cancels an active or pending local connection to a peripheral.
    ///
    /// This method is nonblocking, and any other commands that are still pending to peripheral may
    /// not complete. Because other apps may still have a connection to the peripheral, canceling a
    /// local connection doesn’t guarantee that the underlying physical link is immediately
    /// disconnected. From the app’s perspective, however, the peripheral is effectively
    /// disconnected, and the central manager object trigger
    /// [`PeripheralDisconnected`](enum.CentralEvent.html#variant.PeripheralDisconnected) event.
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

    fn connect(&self, peripheral: &CBPeripheral) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), connectPeripheral:peripheral.as_ptr() options:nil];
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

/// Peripheral's advertisement data.
#[derive(Clone, Debug)]
pub struct AdvertisementData {
    connectable: Option<bool>,
    local_name: Option<String>,
    manufacturer_data: Option<Vec<u8>>,
    service_data: ServiceData,
    service_uuids: Vec<Uuid>,
    solicited_service_uuids: Vec<Uuid>,
    overflow_service_uuids: Vec<Uuid>,
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
