use std::marker::PhantomData;

use crate::UserData;

pub struct LuaRef<T: UserData> {
    ptr: *mut luajit2_sys::lua_State,
    id: i32,
    marker: PhantomData<T>,
}

impl<T: UserData> LuaRef<T> {
    pub fn new(ptr: *mut luajit2_sys::lua_State) -> Self {
        let id = unsafe { luajit2_sys::luaL_ref(ptr, luajit2_sys::LUA_REGISTRYINDEX) };
        LuaRef {
            ptr,
            id,
            marker: PhantomData,
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn to_owned(&self) -> LuaRef<T> {
        unsafe { luajit2_sys::lua_rawgeti(self.ptr, luajit2_sys::LUA_REGISTRYINDEX, self.id) };
        Self::new(self.ptr)
    }

    pub fn as_ref(&self) -> &T {
        unsafe {
            luajit2_sys::lua_rawgeti(self.ptr, luajit2_sys::LUA_REGISTRYINDEX, self.id);
            let ud_ptr = luajit2_sys::lua_touserdata(self.ptr, -1) as *const *const T;
            luajit2_sys::lua_pop(self.ptr, 1);
            &**ud_ptr
        }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe {
            luajit2_sys::lua_rawgeti(self.ptr, luajit2_sys::LUA_REGISTRYINDEX, self.id);
            let ud_ptr = luajit2_sys::lua_touserdata(self.ptr, -1) as *mut *mut T;
            luajit2_sys::lua_pop(self.ptr, 1);
            &mut **ud_ptr
        }
    }
}

impl<T: UserData> Drop for LuaRef<T> {
    fn drop(&mut self) {
        unsafe { luajit2_sys::luaL_unref(self.ptr, luajit2_sys::LUA_REGISTRYINDEX, self.id) };
    }
}
