use luajit2_sys as sys;
use std::marker::PhantomData;

use crate::{UserData, from_lua::FromLua};

#[derive(Debug, Clone)]
pub struct LuaRef<T: FromLua + UserData> {
    ptr: *mut sys::lua_State,
    idx: i32,
    marker: PhantomData<T>,
}

impl<T: FromLua + UserData> LuaRef<T> {
    pub fn new(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self {
            ptr,
            idx,
            marker: PhantomData,
        }
    }
}

impl<T: FromLua + UserData> std::ops::Deref for LuaRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let ud_ptr = sys::lua_touserdata(self.ptr, self.idx) as *mut *mut T;
            &*(*ud_ptr)
        }
    }
}

impl<T: FromLua + UserData> std::ops::DerefMut for LuaRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let ud_ptr = sys::lua_touserdata(self.ptr, self.idx) as *mut *mut T;
            &mut *(*ud_ptr)
        }
    }
}
