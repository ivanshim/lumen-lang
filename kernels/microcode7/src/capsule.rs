use std::ffi::{c_void, CStr, CString};

/// A native resource kept alive by a capsule value, never by a Python instance.
#[derive(Debug)]
pub struct PointerCapsule {
    caption: Option<CString>,
    payload: std::ptr::NonNull<c_void>,
    dispose: Option<unsafe extern "C" fn(*mut c_void)>,
}

impl PointerCapsule {
    /// # Safety
    /// Keep the pointee alive until disposal; a disposal callback must own that pointee.
    pub unsafe fn take(payload: *mut c_void, caption: Option<CString>, dispose: Option<unsafe extern "C" fn(*mut c_void)>) -> Result<Self, &'static str> {
        match std::ptr::NonNull::new(payload) {
            Some(payload) => Ok(Self { caption, payload, dispose }),
            None => Err("PyCapsule_New called with null pointer"),
        }
    }

    pub fn find_pointer(&self, caption: Option<&CStr>) -> Result<*mut c_void, &'static str> {
        match (caption, self.caption.as_deref()) {
            (None, None) => Ok(self.payload.as_ptr()),
            (Some(asked), Some(kept)) if asked == kept => Ok(self.payload.as_ptr()),
            _ => Err("PyCapsule_GetPointer called with incorrect name"),
        }
    }

    pub fn written(&self) -> String {
        match &self.caption {
            None => format!("<capsule object NULL at {self:p}>"),
            Some(name) => format!("<capsule object \"{}\" at {self:p}>", name.to_string_lossy()),
        }
    }
}

impl Drop for PointerCapsule {
    fn drop(&mut self) {
        match self.dispose { Some(dispose) => unsafe { dispose(self.payload.as_ptr()); }, None => () }
    }
}
