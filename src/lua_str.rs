use std::{ffi::CStr, rc::Rc, slice, str::Utf8Error};

use crate::sys;

use crate::{from_lua::FromLua, to_lua::ToLua};

#[derive(Debug)]
struct Inner(*mut sys::lua_State, i32);

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe { sys::luaL_unref(self.0, sys::LUA_REGISTRYINDEX, self.1) };
    }
}

#[derive(Debug, Clone)]
pub struct LuaStr(Rc<Inner>);

impl LuaStr {
    pub fn new(ptr: *mut sys::lua_State, value: &str) -> Self {
        value.to_lua(ptr);
        let id = unsafe { sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX) };
        Self(Rc::new(Inner(ptr, id)))
    }

    pub(crate) fn from_stack(ptr: *mut sys::lua_State, idx: i32) -> Result<Self, Utf8Error> {
        unsafe {
            let ptr = sys::lua_tostring(ptr, idx);
            let cstr = CStr::from_ptr(ptr);
            let _ = cstr.to_str()?;
        }

        unsafe { sys::lua_pushvalue(ptr, idx) };
        let id = unsafe { sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX) };
        Ok(Self(Rc::new(Inner(ptr, id))))
    }

    pub(crate) fn id(&self) -> i32 {
        self.0.1
    }

    pub fn as_bytes(&self) -> &[u8] {
        let ptr = self.0.0;
        let id = self.0.1;

        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, id as _) };
        let mut len: usize = 0;
        let str_ptr = unsafe { sys::lua_tolstring(ptr, -1, &mut len) };
        let slice = unsafe { slice::from_raw_parts(str_ptr as *const u8, len) };
        unsafe { sys::lua_pop(ptr, 1) };

        slice
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(self.as_bytes()).unwrap()
    }
}

impl FromLua for LuaStr {
    type Output = LuaStr;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            LuaStr::from_stack(ptr, idx).ok()
        } else {
            None
        }
    }
}

impl ToLua for LuaStr {
    fn to_lua(self, ptr: *mut crate::sys::lua_State) {
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.id() as _) };
    }
}
