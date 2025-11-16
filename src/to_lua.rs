use std::cell::RefCell;

use luajit2_sys as sys;
use macros::generate_to_lua_tuple_impl;

use crate::{Nil, UserData, lua_ref::LuaRef, table::Table};

pub trait ToLua {
    fn to_lua(self, ptr: *mut sys::lua_State);

    fn len() -> i32 {
        1
    }
}

impl ToLua for i32 {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        unsafe { sys::lua_pushinteger(ptr, self as _) }
    }
}

impl ToLua for f32 {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        unsafe { sys::lua_pushnumber(ptr, self as _) }
    }
}

impl ToLua for f64 {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        unsafe { sys::lua_pushnumber(ptr, self) }
    }
}

impl ToLua for bool {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        unsafe { sys::lua_pushboolean(ptr, if self { 1 } else { 0 }) }
    }
}

impl ToLua for &str {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        unsafe { sys::lua_pushlstring(ptr, self.as_bytes().as_ptr() as *const i8, self.len()) }
    }
}

impl ToLua for String {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        self.as_str().to_lua(ptr)
    }
}

impl ToLua for Nil {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        unsafe { sys::lua_pushnil(ptr) }
    }
}

impl<T> ToLua for T
where
    T: UserData,
{
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        let size = std::mem::size_of::<*mut RefCell<T>>();
        let name = T::name();
        let methods = T::functions();
        let self_ptr = Box::into_raw(Box::new(RefCell::new(self)));

        unsafe {
            let managed_ptr = sys::lua_newuserdata(ptr, size) as *mut *mut RefCell<T>;
            *managed_ptr = self_ptr;

            if sys::luaL_newmetatable(ptr, name) != 0 {
                let mt_idx = sys::lua_gettop(ptr);

                sys::lua_pushstring(ptr, name);
                sys::lua_setfield(ptr, mt_idx, c"__name".as_ptr());

                unsafe extern "C" fn __gc<T: UserData>(ptr: *mut luajit2_sys::lua_State) -> i32 {
                    unsafe {
                        let ud_ptr = sys::lua_touserdata(ptr, 1) as *mut *mut RefCell<T>;
                        if !ud_ptr.is_null() && !(*ud_ptr).is_null() {
                            std::mem::drop(Box::from_raw(*ud_ptr));
                        }
                    }
                    0
                }
                sys::lua_pushcclosure(ptr, Some(__gc::<T>), 0);
                sys::lua_setfield(ptr, mt_idx, c"__gc".as_ptr());

                sys::lua_newtable(ptr);
                sys::luaL_register(ptr, std::ptr::null(), methods.as_ptr());
                sys::lua_setfield(ptr, mt_idx, c"__index".as_ptr());
            }

            sys::lua_setmetatable(ptr, -2);
        }
    }
}

impl ToLua for () {
    fn to_lua(self, _ptr: *mut luajit2_sys::lua_State) {}

    fn len() -> i32 {
        0
    }
}

impl<T: UserData> ToLua for LuaRef<T> {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.id()) };
    }
}

impl ToLua for Table {
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.id()) };
    }
}

impl<T> ToLua for Option<T>
where
    T: ToLua,
{
    fn to_lua(self, ptr: *mut luajit2_sys::lua_State) {
        match self {
            Some(value) => value.to_lua(ptr),
            None => unsafe { sys::lua_pushnil(ptr) },
        }
    }
}

generate_to_lua_tuple_impl!();
