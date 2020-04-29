use super::*;

#[derive(Clone, Debug)]
pub struct Descriptor {
    id: Uuid,
    pub(in crate) descriptor: StrongPtr<CBDescriptor>,
}

assert_impl_all!(Descriptor: Send, Sync);

impl Descriptor {
    pub(in crate) unsafe fn retain(o: impl ObjectPtr) -> Self {
        let descriptor = CBDescriptor::wrap(o).retain();
        Self {
            id: descriptor.id(),
            descriptor,
        }
    }
}

object_ptr_wrapper!(CBDescriptor);

impl CBDescriptor {
    pub fn id(&self) -> Uuid {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), UUID];
            CBUUID::wrap(r).to_uuid()
        }
    }

    pub fn value(&self) -> Option<Vec<u8>> {
        unsafe {
            let r: *mut Object = msg_send![self.as_ptr(), value];
            let r = NSData::wrap_nullable(r)?;
            Some(r.as_bytes().into())
        }
    }
}