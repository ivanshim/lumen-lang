use std::ffi::{c_void, CStr, CString};
use std::ptr::NonNull;

/// An opaque native pointer, with an owned name and an optional finalizer.
#[derive(Debug)]
pub struct NativeCapsule {
    address: NonNull<c_void>,
    label: Option<CString>,
    finalizer: Option<unsafe extern "C" fn(*mut c_void)>,
}

impl NativeCapsule {
    /// # Safety
    /// The pointer must outlive the capsule, or be exclusively owned by its finalizer.
    pub unsafe fn new(address: *mut c_void, label: Option<&CStr>, finalizer: Option<unsafe extern "C" fn(*mut c_void)>) -> Result<Self, &'static str> {
        let address = NonNull::new(address).ok_or("PyCapsule_New called with null pointer")?;
        Ok(Self { address, label: label.map(CStr::to_owned), finalizer })
    }

    pub fn pointer(&self, expected: Option<&CStr>) -> Result<NonNull<c_void>, &'static str> {
        if self.label.as_deref() != expected { return Err("PyCapsule_GetPointer called with incorrect name"); }
        Ok(self.address)
    }

    pub fn representation(&self) -> String {
        let label = self.label.as_ref().map_or_else(|| "NULL".to_owned(), |name| format!("\"{}\"", name.to_string_lossy()));
        format!("<capsule object {label} at {:p}>", self)
    }
}

impl Drop for NativeCapsule {
    fn drop(&mut self) {
        if let Some(release) = self.finalizer { unsafe { release(self.address.as_ptr()); } }
    }
}
