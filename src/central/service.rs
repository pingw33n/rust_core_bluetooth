use super::*;
use crate::central::characteristic::Characteristic;

object_ptr_wrapper!(CBService);

impl CBService {
    pub fn id(&self) -> Uuid {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), UUID];
            CBUUID::wrap(r).to_uuid()
        }
    }

    pub fn is_primary(&self) -> bool {
        unsafe {
            let r: bool = msg_send![self.as_ptr(), isPrimary];
            r
        }
    }

    pub fn characteristics(&self) -> Option<Vec<Characteristic>> {
        let arr = unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), characteristics];
            NSArray::wrap_nullable(r)?
        };
        Some(arr.iter()
            .map(|v| unsafe { Characteristic::retain(v) })
            .collect())
    }
}

#[derive(Clone, Debug)]
pub struct Service {
    id: Uuid,
    primary: bool,
    pub(in crate) service: StrongPtr<CBService>,
}

assert_impl_all!(Service: Send, Sync);

impl Service {
    pub(in crate) unsafe fn retain(o: impl ObjectPtr) -> Self {
        let service = CBService::wrap(o).retain();
        Self {
            id: service.id(),
            primary: service.is_primary(),
            service,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Indicates whether the type of service is primary or secondary.
    ///
    /// A peripheral’s service is either primary or secondary. A primary service describes the
    /// primary function of a device. A secondary service describes a service that’s relevant only
    /// in the context of another service that references it. For example, the primary service of a
    /// heart rate monitor may be to expose heart rate data from the monitor’s heart rate sensor.
    /// In this example, a secondary service may be to expose the sensor’s battery data.
    pub fn is_primary(&self) -> bool {
        self.primary
    }
}

impl PartialEq for Service {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Service {}

impl std::hash::Hash for Service {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.id)
    }
}