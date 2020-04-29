macro_rules! object_ptr_wrapper {
    ($n:ident) => {
        #[derive(Clone, Copy, Debug)]
        #[repr(transparent)]
        pub(in crate) struct $n(::std::ptr::NonNull<::objc::runtime::Object>);

        impl $n {
            #[allow(dead_code)]
            pub unsafe fn wrap(v: impl crate::platform::ObjectPtr) -> Self {
                Self(::std::ptr::NonNull::new(v.as_ptr()).unwrap())
            }

            #[allow(dead_code)]
            pub unsafe fn wrap_nullable(v: *mut ::objc::runtime::Object) -> Option<Self> {
                ::std::ptr::NonNull::new(v).map(|v| Self(v))
            }

            #[allow(dead_code)]
            pub fn retain(self) -> crate::platform::StrongPtr<Self> {
                unsafe { crate::platform::StrongPtr::retain(self) }
            }
        }

        impl crate::platform::ObjectPtr for $n {
            fn as_ptr(&self) -> *mut ::objc::runtime::Object {
                self.0.as_ptr()
            }
        }

        unsafe impl ::std::marker::Send for $n {}
        unsafe impl ::std::marker::Sync for $n {}
    };
}