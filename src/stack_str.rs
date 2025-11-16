use luajit2_sys as sys;
use std::{ffi::CStr, str::Utf8Error};

#[derive(Debug)]
pub struct StackStr(*mut sys::lua_State, i32);

impl StackStr {
    pub fn new(ptr: *mut sys::lua_State, idx: i32) -> Result<Self, Utf8Error> {
        unsafe {
            let ptr = sys::lua_tostring(ptr, idx);
            let cstr = CStr::from_ptr(ptr);
            let _ = cstr.to_str()?;
        }

        Ok(Self(ptr, idx))
    }

    pub fn as_str(&self) -> &str {
        let (ptr, idx) = (self.0, self.1);

        unsafe {
            let ptr = sys::lua_tostring(ptr, idx);
            let cstr = CStr::from_ptr(ptr);
            let s = cstr.to_str().unwrap_unchecked();
            s
        }
    }
}
