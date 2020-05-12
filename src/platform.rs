use objc::*;
use objc::runtime::*;
use std::ffi::CStr;
use std::os::raw::*;
use std::ptr::{self, NonNull};

use std::cell::Cell;

#[allow(non_upper_case_globals)]
pub const nil: *mut Object = ptr::null_mut();

pub type NSInteger = isize;
pub type NSUInteger = usize;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "Foundation", kind = "framework")]
#[link(name = "CoreBluetooth", kind = "framework")]
extern {
    pub(in crate) static CBAdvertisementDataIsConnectable: NSString;
    pub(in crate) static CBAdvertisementDataLocalNameKey: NSString;
    pub(in crate) static CBAdvertisementDataManufacturerDataKey: NSString;
    pub(in crate) static CBAdvertisementDataOverflowServiceUUIDsKey: NSString;
    pub(in crate) static CBAdvertisementDataServiceDataKey: NSString;
    pub(in crate) static CBAdvertisementDataServiceUUIDsKey: NSString;
    pub(in crate) static CBAdvertisementDataSolicitedServiceUUIDsKey: NSString;
    pub(in crate) static CBAdvertisementDataTxPowerLevelKey: NSString;
    pub(in crate) static CBCentralManagerScanOptionAllowDuplicatesKey: NSString;
    pub(in crate) static CBCentralManagerScanOptionSolicitedServiceUUIDsKey: NSString;
    pub(in crate) static CBCentralManagerOptionShowPowerAlertKey: NSString;
    pub(in crate) static CBErrorDomain: NSString;
    pub(in crate) static CBATTErrorDomain: NSString;
}

pub trait ObjectPtr {
    unsafe fn retain_count(&self) -> usize {
        let r: usize = msg_send![self.as_ptr(), retainCount];
        r
    }

    unsafe fn class(&self) -> &'static Class {
        &*object_getClass(self.as_ptr())
    }

    unsafe fn ivar(&self, name: &str) -> *mut c_void {
        *self.as_ptr().as_ref().unwrap().get_ivar::<*mut c_void>(name)
    }

    unsafe fn ivar_mut(&mut self, name: &str) -> &mut *mut c_void {
        self.as_ptr().as_mut().unwrap().get_mut_ivar::<*mut c_void>(name)
    }

    fn as_ptr(&self) -> *mut Object;
}

impl ObjectPtr for &mut Object {
    fn as_ptr(&self) -> *mut Object {
        *self as *const Object as *mut _
    }
}

impl ObjectPtr for *mut Object {
    fn as_ptr(&self) -> *mut Object {
        *self
    }
}

impl ObjectPtr for NonNull<Object> {
    fn as_ptr(&self) -> *mut Object {
        NonNull::as_ptr(*self)
    }
}

impl<T: ObjectPtr> ObjectPtr for Option<T> {
    fn as_ptr(&self) -> *mut Object {
        self.as_ref().map(|v| v.as_ptr()).unwrap_or(nil)
    }
}

#[derive(Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct StrongPtr<T: ObjectPtr>(T);

impl<T: ObjectPtr> StrongPtr<T> {
    pub unsafe fn wrap(inner: T) -> Self {
        Self(inner)
    }

    pub unsafe fn retain(inner: T) -> Self {
        objc_retain(inner.as_ptr());
        Self(inner)
    }
}

impl<T: ObjectPtr> std::ops::Deref for StrongPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ObjectPtr + Clone> Clone for StrongPtr<T> {
    fn clone(&self) -> Self {
        unsafe { objc_retain(self.as_ptr()); }
        Self(self.0.clone())
    }
}

impl<T: ObjectPtr> Drop for StrongPtr<T> {
    fn drop(&mut self) {
        unsafe { objc_release(self.as_ptr()); }
    }
}

impl<T: ObjectPtr + std::fmt::Debug> std::fmt::Debug for StrongPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl<T: ObjectPtr> ObjectPtr for StrongPtr<T> {
    fn as_ptr(&self) -> *mut Object {
        self.0.as_ptr()
    }
}

#[allow(non_camel_case_types)]
pub type dispatch_function_t = extern fn(*mut c_void);

pub const DISPATCH_QUEUE_SERIAL: *mut Object = ptr::null_mut();

extern "C" {
    pub fn dispatch_async_f(queue: *mut Object, context: *mut c_void, work: dispatch_function_t);
    pub fn dispatch_queue_create(label: *const c_char, attr: *mut Object) -> *mut Object;
}

object_ptr_wrapper!(NSNumber);

impl NSNumber {
    pub fn new_bool(value: bool) -> Self {
        unsafe {
            let r: *mut Object = msg_send![class!(NSNumber), numberWithBool:value];
            Self::wrap(r)
        }
    }

    pub fn get_bool(&self) -> bool {
        unsafe {
            let r: bool = msg_send![self.as_ptr(), boolValue];
            r
        }
    }

    pub fn get_i32(&self) -> i32 {
        unsafe {
            let r: i32 = msg_send![self.as_ptr(), intValue];
            r
        }
    }
}

object_ptr_wrapper!(NSString);

impl NSString {
    pub fn as_str(&self) -> &str {
        unsafe {
            let r: *const c_char = msg_send![self.as_ptr(), UTF8String];
            &*CStr::from_ptr(r).to_str().unwrap()
        }
    }

    pub fn is_equal_to_string(&self, s: NSString) -> bool {
        unsafe {
            let r: bool = msg_send![self.as_ptr(), isEqualToString:s];
            r
        }
    }
}

object_ptr_wrapper!(NSData);

impl NSData {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        unsafe {
            let r: *mut Object = msg_send![class!(NSData),
                dataWithBytes:bytes.as_ptr() length:bytes.len()];
            Self::wrap(r)
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let len: usize = msg_send![self.as_ptr(), length];
            if len > 0 {
                let r: *const c_void = msg_send![self.as_ptr(), bytes];
                std::slice::from_raw_parts(r as *const u8, len)
            } else {
                &[]
            }
        }
    }
}

object_ptr_wrapper!(NSArray);

impl NSArray {
    pub fn with_capacity(capacity: NSUInteger) -> Self {
        unsafe {
            let r: *mut Object = msg_send![class!(NSMutableArray), arrayWithCapacity:capacity];
            Self::wrap(r)
        }
    }

    pub fn from_iter<T, I>(iter: I) -> Self
    where T: ObjectPtr,
          I: Iterator<Item=T>
    {
        let r = NSArray::with_capacity(iter.size_hint().0);
        for v in iter {
            r.push(v);
        }
        r
    }

    pub fn push(&self, obj: impl ObjectPtr) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), addObject:obj];
        }
    }

    pub fn iter(&self) -> NSEnumerator<Self> {
        let en = unsafe {
            let en: *mut Object = msg_send![self.as_ptr(), objectEnumerator];
            en
        };
        NSEnumerator {
            owner: self,
            en,
            count: Default::default(),
        }
    }
}

object_ptr_wrapper!(NSDictionary);

impl NSDictionary {
    pub fn with_capacity(capacity: NSUInteger) -> Self {
        unsafe {
            let r: *mut Object = msg_send![class!(NSMutableDictionary), dictionaryWithCapacity:capacity];
            Self::wrap(r)
        }
    }

    pub fn get(&self, key: impl ObjectPtr) -> Option<NonNull<Object>> {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), objectForKey:key.as_ptr()];
            NonNull::new(r)
        }
    }

    pub fn insert(&self, key: impl ObjectPtr, value: impl ObjectPtr) {
        unsafe {
            let _: () = msg_send![self.as_ptr(), setObject:value forKey:key];
        }
    }

    pub fn keys(&self) -> NSEnumerator<Self> {
        let en = unsafe {
            let en: *mut Object = msg_send![self.as_ptr(), keyEnumerator];
            en
        };
        NSEnumerator {
            owner: self,
            en,
            count: Default::default(),
        }
    }

    pub fn iter(&self) -> NSDictionaryIter {
        NSDictionaryIter {
            keys: self.keys(),
        }
    }
}

pub struct NSDictionaryIter<'a> {
    keys: NSEnumerator<'a, NSDictionary>,
}

impl Iterator for NSDictionaryIter<'_> {
    type Item = (NonNull<Object>, NonNull<Object>);

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.keys.next()?;
        let value = self.keys.owner.get(key).unwrap();
        Some((key, value))
    }
}

pub struct NSEnumerator<'a, T> {
    owner: &'a T,
    en: *mut Object,
    count: Cell<Option<NSUInteger>>,
}

impl<T: ObjectPtr> Iterator for NSEnumerator<'_, T> {
    type Item = NonNull<Object>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let r: *mut Object = msg_send![self.en.as_ptr(), nextObject];
            NonNull::new(r)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.count.get().is_none() {
            let v = unsafe {
                let v: NSUInteger = msg_send![self.owner.as_ptr(), count];
                v
            };
            self.count.set(Some(v));
        }
        let c = self.count.get().unwrap();
        (c, Some(c))
    }

    fn count(self) -> usize
        where Self: Sized
    {
        self.size_hint().0
    }
}

object_ptr_wrapper!(NSError);

impl NSError {
    pub fn code(&self) -> NSInteger {
        unsafe {
            let r: NSInteger = msg_send![self.as_ptr(), code];
            r
        }
    }

    pub fn domain(&self) -> NSString {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), domain];
            NSString::wrap(r)
        }
    }

    pub fn description(&self) -> NSString {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), localizedDescription];
            NSString::wrap(r)
        }
    }
}



