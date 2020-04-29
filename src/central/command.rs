use super::*;
use super::characteristic::{CBCharacteristic, WriteKind};
use super::descriptor::CBDescriptor;
use super::service::CBService;

macro_rules! impl_via_manager {
    ($ctx_ty:ident => $($n:ident ( $ctx:ident ) $code:expr)*) => {
        impl $ctx_ty {
            $(
            pub fn $n(self) {
                extern fn f(ctx: *mut c_void) {
                    unsafe {
                        let $ctx = $ctx_ty::from_ctx(ctx);
                        $code;
                    }
                }
                unsafe {
                    let queue = self.manager.delegate().queue();
                    Command::dispatch(self, queue, f);
                }
            }
            )*
        }
    };
}

macro_rules! impl_via_peripheral {
    ($ctx_ty:ident => $($n:ident ( $ctx:ident ) $code:expr)*) => {
        impl $ctx_ty {
            $(
            pub fn $n(self) {
                extern fn f(ctx: *mut c_void) {
                    unsafe {
                        let $ctx = $ctx_ty::from_ctx(ctx);
                        $code;
                    }
                }
                unsafe {
                    let queue = self.peripheral.delegate().queue();
                    Command::dispatch(self, queue, f);
                }
            }
            )*
        }
    };
}

pub trait Command: 'static + Sized + Send  {
    fn into_ctx(self) -> *mut c_void {
        Box::into_raw(Box::new(self)) as *mut c_void
    }

    unsafe fn from_ctx(v: *mut c_void) -> Self {
        *Box::from_raw(v as *mut Self)
    }

    unsafe fn dispatch(self, queue: *mut Object, f: dispatch_function_t) {
        dispatch_async_f(queue, self.into_ctx(), f);
    }
}

#[repr(transparent)]
pub struct Manager {
    pub(in super) manager: StrongPtr<CBCentralManager>,
}

impl Command for Manager {
    fn into_ctx(self) -> *mut c_void {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_ctx(v: *mut c_void) -> Self {
        mem::transmute(v)
    }
}

impl_via_manager! { Manager =>
    cancel_scan(ctx) {
        ctx.manager.cancel_scan();
    }
    drop_self(ctx) {
        ctx.manager.drop_self();
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct GetPeripherals {
    pub(in super) manager: StrongPtr<CBCentralManager>,
    pub(in super) uuids: StrongPtr<NSArray>,
    pub(in super) tag: Option<Tag>,
}

impl Command for GetPeripherals {}

impl_via_manager! { GetPeripherals =>
    get_peripherals(ctx) {
        let peripherals = ctx.manager.get_peripherals(*ctx.uuids).unwrap_or_default();
        ctx.manager.delegate().send(CentralEvent::GetPeripheralsResult {
            peripherals,
            tag: ctx.tag,
        });
    }
    get_peripherals_with_services(ctx) {
        let peripherals = ctx.manager.get_peripherals_with_services(*ctx.uuids).unwrap_or_default();
        ctx.manager.delegate().send(CentralEvent::GetPeripheralsWithServicesResult {
            peripherals,
            tag: ctx.tag,
        });
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct CancelConnect {
    pub(in super) manager: StrongPtr<CBCentralManager>,
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
}

impl Command for CancelConnect {}

impl_via_manager! { CancelConnect =>
    cancel_connect(ctx) {
        ctx.manager.cancel_connect(&ctx.peripheral);
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct Scan {
    pub(in super) manager: StrongPtr<CBCentralManager>,
    pub(in super) options: ScanOptions,
}

impl Command for Scan {}

impl_via_manager! { Scan =>
    dispatch(ctx) {
        ctx.manager.scan(&ctx.options);
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct Connect {
    pub(in super) manager: StrongPtr<CBCentralManager>,
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
    pub(in super) options: ConnectOptions,
}

impl Command for Connect {}

impl_via_manager! { Connect =>
    dispatch(ctx) {
        ctx.manager.connect(&ctx.peripheral, &ctx.options);
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct DiscoverServices {
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
    pub(in super) uuids: Option<StrongPtr<NSArray>>,
}

impl Command for DiscoverServices {}

impl_via_peripheral! { DiscoverServices =>
    dispatch(ctx) {
        ctx.peripheral.discover_services(ctx.uuids.as_ref().map(|v| **v));
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct DiscoverCharacteristics {
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
    pub(in super) service: StrongPtr<CBService>,
    pub(in super) uuids: Option<StrongPtr<NSArray>>,
}

impl Command for DiscoverCharacteristics {}

impl_via_peripheral! { DiscoverCharacteristics =>
    dispatch(ctx) {
        ctx.peripheral.discover_characteristics(*ctx.service, ctx.uuids.as_ref().map(|v| **v));
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct Peripheral {
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
}

impl Command for Peripheral {
    fn into_ctx(self) -> *mut c_void {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_ctx(v: *mut c_void) -> Self {
        mem::transmute(v)
    }
}

impl_via_peripheral! { Peripheral =>
    read_rssi(ctx) {
        ctx.peripheral.read_rssi();
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct PeripheralTag {
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
    pub(in super) tag: Option<Tag>,
}

impl Command for PeripheralTag {}

impl_via_peripheral! { PeripheralTag =>
    get_max_write_len(ctx) {
        let with_response = ctx.peripheral.max_write_len(WriteKind::WithResponse);
        let without_response = ctx.peripheral.max_write_len(WriteKind::WithoutResponse);
        let max_write_len = MaxWriteLen {
            with_response,
            without_response,
        };
        ctx.peripheral.delegate().send(CentralEvent::GetMaxWriteLenResult {
            max_write_len,
            tag: ctx.tag,
        });
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct Characteristic {
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
    pub(in super) characteristic: StrongPtr<CBCharacteristic>,
}

impl Command for Characteristic {}

impl_via_peripheral! { Characteristic =>
    discover_descriptors(ctx) {
        ctx.peripheral.discover_descriptors(*ctx.characteristic);
    }
    read(ctx) {
        ctx.peripheral.read_characteristic(*ctx.characteristic);
    }
    subscribe(ctx) {
        ctx.peripheral.set_notify_value(*ctx.characteristic, true);
    }
    unsubscribe(ctx) {
        ctx.peripheral.set_notify_value(*ctx.characteristic, false);
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct WriteCharacteristic {
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
    pub(in super) characteristic: StrongPtr<CBCharacteristic>,
    pub(in super) value: StrongPtr<NSData>,
    pub(in super) kind: WriteKind,
}

impl Command for WriteCharacteristic {}

impl_via_peripheral! { WriteCharacteristic =>
    dispatch(ctx) {
        ctx.peripheral.write_characteristic(*ctx.characteristic, *ctx.value, ctx.kind);
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct Descriptor {
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
    pub(in super) descriptor: StrongPtr<CBDescriptor>,
}

impl Command for Descriptor {}

impl_via_peripheral! { Descriptor =>
    read(ctx) {
        ctx.peripheral.read_descriptor(*ctx.descriptor);
    }
}

///////////////////////////////////////////////////////////////////////////////////

pub struct WriteDescriptor {
    pub(in super) peripheral: StrongPtr<CBPeripheral>,
    pub(in super) descriptor: StrongPtr<CBDescriptor>,
    pub(in super) value: StrongPtr<NSData>,
}

impl Command for WriteDescriptor {}

impl_via_peripheral! { WriteDescriptor =>
    dispatch(ctx) {
        ctx.peripheral.write_descriptor(*ctx.descriptor, *ctx.value);
    }
}