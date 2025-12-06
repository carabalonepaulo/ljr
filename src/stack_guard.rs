use crate::sys;

pub struct StackGuard(*mut sys::lua_State, i32);

impl StackGuard {
    pub(crate) fn new(ptr: *mut sys::lua_State) -> Self {
        Self(ptr, unsafe { sys::lua_gettop(ptr) })
    }
}

impl Drop for StackGuard {
    fn drop(&mut self) {
        unsafe { sys::lua_settop(self.0, self.1) };
    }
}
