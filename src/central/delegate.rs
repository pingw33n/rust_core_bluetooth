use lazy_static::lazy_static;
use log::*;
use objc::*;
use objc::declare::ClassDecl;
use objc::runtime::*;
use std::os::raw::*;
use std::ptr;
use std::ptr::NonNull;

use super::*;
use crate::central::peripheral::Peripheral;
use crate::error::*;
use crate::platform::*;

const QUEUE_IVAR: &'static str = "__queue";
const SENDER_IVAR: &'static str = "__sender";

type Sender = crate::sync::Sender<CentralEvent>;

object_ptr_wrapper!(Delegate);

impl Delegate {
    pub fn new(sender: Sender, queue: *mut Object) -> StrongPtr<Self> {
        let mut r = unsafe {
            let r: *mut Object = msg_send![*DELEGATE_CLASS, alloc];
            Self::wrap(r)
        };
        r.set_sender(sender);
        r.set_queue(queue);
        unsafe { StrongPtr::wrap(r) }
    }

    pub fn drop_self(&mut self) {
        trace!("dropping delegate {:?}", self.0);
        self.drop_sender();
    }

    pub fn queue(&self) -> *mut Object {
        unsafe {
            self.ivar(QUEUE_IVAR) as *mut Object
        }
    }

    fn set_queue(&mut self, queue: *mut Object) {
        unsafe {
            *self.ivar_mut(QUEUE_IVAR) = queue as *mut c_void;
        }
    }

    fn sender(&self) -> Option<&Sender> {
        unsafe {
            (self.ivar(SENDER_IVAR) as *mut Sender).as_ref()
        }
    }

    fn set_sender(&mut self, sender: Sender) {
        unsafe {
            *self.ivar_mut(SENDER_IVAR) = Box::into_raw(Box::new(sender)) as *mut c_void;
        }
    }

    fn drop_sender(&mut self) {
        unsafe {
            let p = self.ivar_mut(SENDER_IVAR);
            let _ = Box::<Sender>::from_raw(NonNull::new(*p).unwrap().as_ptr() as *mut Sender);
            *p = ptr::null_mut();
        }
    }

    pub fn send(&self, event: CentralEvent) {
        if let Some(sender) = self.sender() {
            let _ = sender.send_blocking(event);
        }
    }

    #[allow(non_snake_case)]
    extern fn centralManager_didConnectPeripheral(
        this: &mut Object,
        _: Sel,
        _manager: *mut Object,
        peripheral: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);

            this.send(CentralEvent::PeripheralConnected {
                peripheral,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn centralManager_didDisconnectPeripheral_error(
        this: &mut Object,
        _: Sel,
        _manager: *mut Object,
        peripheral: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let error = NSError::wrap_nullable(error).map(Error::from_ns_error);
            this.send(CentralEvent::PeripheralDisconnected {
                peripheral,
                error,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn centralManager_didFailToConnectPeripheral_error(
        this: &mut Object,
        _: Sel,
        _manager: *mut Object,
        peripheral: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let error = NSError::wrap_nullable(error).map(Error::from_ns_error);
            this.send(CentralEvent::PeripheralConnectFailed {
                peripheral,
                error,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn centralManager_didDiscoverPeripheral_advertisementData_RSSI(
        this: &mut Object,
        _: Sel,
        _manager: *mut Object,
        peripheral: *mut Object,
        advertisement_data: *mut Object,
        rssi: *mut Object)
    {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let advertisement_data = AdvertisementData::from_dict(NSDictionary::wrap(advertisement_data));
            let rssi = NSNumber::wrap(rssi).get_i32();

            peripheral.peripheral.set_delegate(this);

            this.send(CentralEvent::PeripheralDiscovered {
                peripheral,
                advertisement_data,
                rssi,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn centralManagerDidUpdateState(this: &mut Object, _: Sel, manager: *mut Object) {
        unsafe {
            let this = Delegate::wrap(this);
            let new_state = CBCentralManager::wrap(manager).state();

            this.send(CentralEvent::ManagerStateChanged { new_state });
        }
    }

    #[allow(non_snake_case)]
    extern fn centralManager_didUpdateANCSAuthorizationForPeripheral(
        _this: &mut Object,
        _: Sel,
        _manager: *mut Object,
        _peripheral: *mut Object,
    ) {
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didDiscoverServices(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let services = result(
                NSError::wrap_nullable(error), || peripheral.peripheral.services().unwrap());
            this.send(CentralEvent::ServicesDiscovered {
                peripheral,
                services,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didDiscoverIncludedServicesForService_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        service: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let service = Service::retain(service);
            let included_services = result(
                NSError::wrap_nullable(error), || peripheral.peripheral.included_services().unwrap());
            this.send(CentralEvent::IncludedServicesDiscovered {
                peripheral,
                service,
                included_services,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didDiscoverCharacteristicsForService_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        service: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let service = Service::retain(service);
            let characteristics = result(
                NSError::wrap_nullable(error), || service.service.characteristics().unwrap());
            this.send(CentralEvent::CharacteristicsDiscovered {
                peripheral,
                service,
                characteristics,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didDiscoverDescriptorsForCharacteristic_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        characteristic: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let characteristic = Characteristic::retain(characteristic);
            let descriptors = result(
                NSError::wrap_nullable(error), || characteristic.characteristic.descriptors().unwrap());
            this.send(CentralEvent::DescriptorsDiscovered {
                peripheral,
                characteristic,
                descriptors,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didUpdateValueForCharacteristic_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        characteristic: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let characteristic = Characteristic::retain(characteristic);
            let value = result(NSError::wrap_nullable(error),
                || characteristic.characteristic.value().unwrap());
            this.send(CentralEvent::CharacteristicValue {
                peripheral,
                characteristic,
                value,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didUpdateValueForDescriptor_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        descriptor: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let descriptor = Descriptor::retain(descriptor);
            let value = result(NSError::wrap_nullable(error),
                || descriptor.descriptor.value().unwrap());
            this.send(CentralEvent::DescriptorValue {
                peripheral,
                descriptor,
                value,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didWriteValueForCharacteristic_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        characteristic: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let characteristic = Characteristic::retain(characteristic);
            let result = result(NSError::wrap_nullable(error), || {});
            this.send(CentralEvent::WriteCharacteristicResult {
                peripheral,
                characteristic,
                result,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didWriteValueForDescriptor_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        descriptor: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let descriptor = Descriptor::retain(descriptor);
            let result = result(NSError::wrap_nullable(error), || {});
            this.send(CentralEvent::WriteDescriptorResult {
                peripheral,
                descriptor,
                result,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheralIsReadyToSendWriteWithoutResponse(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            this.send(CentralEvent::PeripheralIsReadyToWriteWithoutResponse {
                peripheral,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didUpdateNotificationStateForCharacteristic_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        characteristic: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let characteristic = Characteristic::retain(characteristic);
            let result = result(NSError::wrap_nullable(error), || {});
            this.send(CentralEvent::SubscriptionChanged {
                peripheral,
                characteristic,
                result,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didReadRSSI_error(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        rssi: *mut Object,
        error: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let rssi = result(NSError::wrap_nullable(error), || NSNumber::wrap(rssi).get_i32());
            this.send(CentralEvent::ReadRssiResult {
                peripheral,
                rssi,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheralDidUpdateName(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let new_name = peripheral.peripheral.name().map(|s| s.as_str().to_owned());
            this.send(CentralEvent::PeripheralNameChanged {
                peripheral,
                new_name,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didModifyServices(
        this: &mut Object,
        _: Sel,
        peripheral: *mut Object,
        invalidated_services: *mut Object,
    ) {
        unsafe {
            let this = Delegate::wrap(this);
            let peripheral = Peripheral::retain(peripheral);
            let services = peripheral.peripheral.services().unwrap();
            let invalidated_services = NSArray::wrap(invalidated_services)
                .iter()
                .map(|s| Service::retain(s))
                .collect();
            this.send(CentralEvent::ServicesChanged {
                peripheral,
                services,
                invalidated_services,
            });
        }
    }

    #[allow(non_snake_case)]
    extern fn peripheral_didOpenL2CAPChannel_error(
        _this: &mut Object,
        _: Sel,
        _peripheral: *mut Object,
        _channel: *mut Object,
        _error: *mut Object,
    ) {
    }
}

lazy_static! {
    static ref DELEGATE_CLASS: &'static Class = {
        let mut decl = ClassDecl::new("RustCoreBluetoothCentralDelegate", class!(NSObject)).unwrap();
        decl.add_protocol(Protocol::get("CBCentralManagerDelegate").unwrap());
        decl.add_protocol(Protocol::get("CBPeripheralDelegate").unwrap());

        decl.add_ivar::<*mut c_void>(QUEUE_IVAR);
        decl.add_ivar::<*mut c_void>(SENDER_IVAR);

        unsafe {
            type D = Delegate;

            // CBCentralManagerDelegate

            decl.add_method(
                sel!(centralManager:didConnectPeripheral:),
                D::centralManager_didConnectPeripheral as extern fn(&mut Object, Sel, *mut Object, *mut Object));
            decl.add_method(
                sel!(centralManager:didDisconnectPeripheral:error:),
                D::centralManager_didDisconnectPeripheral_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(centralManager:didFailToConnectPeripheral:error:),
                D::centralManager_didFailToConnectPeripheral_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(centralManager:didDiscoverPeripheral:advertisementData:RSSI:),
                D::centralManager_didDiscoverPeripheral_advertisementData_RSSI as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object, *mut Object));
            decl.add_method(sel!(centralManagerDidUpdateState:),
                D::centralManagerDidUpdateState as extern fn(&mut Object, Sel, *mut Object));
            decl.add_method(
                sel!(centralManager:didUpdateANCSAuthorizationForPeripheral:),
                D::centralManager_didUpdateANCSAuthorizationForPeripheral as extern fn(&mut Object, Sel, *mut Object, *mut Object));

            // CBPeripheralDelegate

            decl.add_method(
                sel!(peripheral:didDiscoverServices:),
                D::peripheral_didDiscoverServices as extern fn(&mut Object, Sel, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didDiscoverIncludedServicesForService:error:),
                D::peripheral_didDiscoverIncludedServicesForService_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didDiscoverCharacteristicsForService:error:),
                D::peripheral_didDiscoverCharacteristicsForService_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didDiscoverDescriptorsForCharacteristic:error:),
                D::peripheral_didDiscoverDescriptorsForCharacteristic_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didUpdateValueForCharacteristic:error:),
                D::peripheral_didUpdateValueForCharacteristic_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didUpdateValueForDescriptor:error:),
                D::peripheral_didUpdateValueForDescriptor_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didWriteValueForCharacteristic:error:),
                D::peripheral_didWriteValueForCharacteristic_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didWriteValueForDescriptor:error:),
                D::peripheral_didWriteValueForDescriptor_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheralIsReadyToSendWriteWithoutResponse:),
                D::peripheralIsReadyToSendWriteWithoutResponse as extern fn(&mut Object, Sel, *mut Object));
            decl.add_method(
                sel!(peripheral:didUpdateNotificationStateForCharacteristic:error:),
                D::peripheral_didUpdateNotificationStateForCharacteristic_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didReadRSSI:error:),
                D::peripheral_didReadRSSI_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheralDidUpdateName:),
                D::peripheralDidUpdateName as extern fn(&mut Object, Sel, *mut Object));
            decl.add_method(
                sel!(peripheral:didModifyServices:),
                D::peripheral_didModifyServices as extern fn(&mut Object, Sel, *mut Object, *mut Object));
            decl.add_method(
                sel!(peripheral:didOpenL2CAPChannel:error:),
                D::peripheral_didOpenL2CAPChannel_error as extern fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object));
        }
        decl.register()
    };
}
