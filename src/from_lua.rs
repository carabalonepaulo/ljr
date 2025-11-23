use std::ffi::CStr;

use crate::sys;
use macros::generate_from_lua_tuple_impl;

use crate::table::Table;

pub trait FromLua {
    type Output;

    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self::Output>;

    fn len() -> i32 {
        1
    }
}

impl FromLua for i32 {
    type Output = i32;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) } as i32)
        } else {
            None
        }
    }
}

impl FromLua for f32 {
    type Output = f32;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) } as f32)
        } else {
            None
        }
    }
}

impl FromLua for f64 {
    type Output = f64;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) })
        } else {
            None
        }
    }
}

impl FromLua for bool {
    type Output = bool;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isboolean(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_toboolean(ptr, idx) != 0 })
        } else {
            None
        }
    }
}

impl FromLua for String {
    type Output = String;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        unsafe {
            if sys::lua_type(ptr, idx) == sys::LUA_TSTRING as i32 {
                let ptr = sys::lua_tostring(ptr, idx);
                let cstr = CStr::from_ptr(ptr);
                Some(cstr.to_str().ok()?.to_string())
            } else {
                None
            }
        }
    }
}

impl FromLua for Table {
    type Output = Table;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_istable(ptr, idx) } != 0 {
            Some(Table::from_stack(ptr, idx))
        } else {
            None
        }
    }
}

impl<T> FromLua for Option<T>
where
    T: FromLua,
    T::Output: FromLua,
{
    type Output = Option<T::Output>;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TNIL as i32 {
            Some(None)
        } else {
            <T as FromLua>::from_lua(ptr, idx).map(Some)
        }
    }

    fn len() -> i32 {
        <T as FromLua>::Output::len()
    }
}

impl FromLua for () {
    type Output = ();

    fn from_lua(_: *mut crate::sys::lua_State, _: i32) -> Option<Self::Output> {
        Some(())
    }

    fn len() -> i32 {
        0
    }
}

generate_from_lua_tuple_impl!();
