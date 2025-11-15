use std::{marker::PhantomData, rc::Rc};

use crate::UserData;

#[derive(Debug)]
struct Inner {
    ptr: *mut luajit2_sys::lua_State,
    id: i32,
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe { luajit2_sys::luaL_unref(self.ptr, luajit2_sys::LUA_REGISTRYINDEX, self.id) };
    }
}

#[derive(Debug)]
pub struct LuaRef<T: UserData> {
    inner: Rc<Inner>,
    marker: PhantomData<T>,
}

impl<T: UserData> LuaRef<T> {
    pub fn new(ptr: *mut luajit2_sys::lua_State) -> Self {
        let id = unsafe { luajit2_sys::luaL_ref(ptr, luajit2_sys::LUA_REGISTRYINDEX) };
        LuaRef {
            inner: Rc::new(Inner { ptr, id }),
            marker: PhantomData,
        }
    }

    pub fn id(&self) -> i32 {
        self.inner.id
    }

    pub fn as_ref(&self) -> &T {
        let ptr = self.inner.ptr;
        let id = self.inner.id;

        unsafe {
            luajit2_sys::lua_rawgeti(ptr, luajit2_sys::LUA_REGISTRYINDEX, id);
            let ud_ptr = luajit2_sys::lua_touserdata(ptr, -1) as *const *const T;
            luajit2_sys::lua_pop(ptr, 1);
            &**ud_ptr
        }
    }

    pub fn as_mut(&mut self) -> &mut T {
        let ptr = self.inner.ptr;
        let id = self.inner.id;

        unsafe {
            luajit2_sys::lua_rawgeti(ptr, luajit2_sys::LUA_REGISTRYINDEX, id);
            let ud_ptr = luajit2_sys::lua_touserdata(ptr, -1) as *mut *mut T;
            luajit2_sys::lua_pop(ptr, 1);
            &mut **ud_ptr
        }
    }
}

impl<T: UserData> Clone for LuaRef<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: self.marker.clone(),
        }
    }
}
