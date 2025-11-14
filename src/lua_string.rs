use std::{rc::Rc, slice};

use luajit2_sys as sys;

#[derive(Debug)]
struct Inner(*mut sys::lua_State, i32);

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe { sys::luaL_unref(self.0, sys::LUA_REGISTRYINDEX, self.1) };
    }
}

#[derive(Debug)]
pub struct LuaString(Rc<Inner>);

impl LuaString {
    pub(crate) fn new(ptr: *mut sys::lua_State, idx: i32) -> Self {
        let id = unsafe { sys::luaL_ref(ptr, idx) };
        Self(Rc::new(Inner(ptr, id)))
    }

    pub fn as_bytes(&self) -> &[u8] {
        let ptr = self.0.0;
        let id = self.0.1;

        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, id) };
        let mut len: usize = 0;
        let str_ptr = unsafe { sys::lua_tolstring(ptr, -1, &mut len) };
        let slice = unsafe { slice::from_raw_parts(str_ptr as *const u8, len) };
        unsafe { sys::lua_pop(ptr, 1) };

        slice
    }

    pub fn as_str(&self) -> Option<&str> {
        str::from_utf8(self.as_bytes()).ok()
    }
}
