#![allow(unused)]

use crate::sys;

pub struct StackGuard(*mut sys::lua_State, i32);

impl StackGuard {
    #[inline(always)]
    pub(crate) fn new(ptr: *mut sys::lua_State) -> Self {
        Self(ptr, unsafe { sys::lua_gettop(ptr) })
    }

    pub(crate) fn scope<F, T, E>(ptr: *mut sys::lua_State, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        let g = Self::new(ptr);
        let r = f();

        if r.is_ok() {
            g.commit();
        }

        r
    }

    #[inline(always)]
    pub(crate) fn commit(self) {
        std::mem::forget(self);
    }
}

impl Drop for StackGuard {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { sys::lua_settop(self.0, self.1) };
    }
}
