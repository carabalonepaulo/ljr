use crate::sys;

use crate::{
    AnyLuaFunction, AnyNativeFunction, AnyUserData, Coroutine, LightUserData, Nil, table::Table,
};

pub trait IsType {
    fn is_type(ptr: *mut sys::lua_State, idx: i32) -> bool;
}

impl IsType for i32 {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isnumber(ptr, idx) != 0 }
    }
}

impl IsType for f32 {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isnumber(ptr, idx) != 0 }
    }
}

impl IsType for f64 {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isnumber(ptr, idx) != 0 }
    }
}

impl IsType for bool {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isboolean(ptr, idx) != 0 }
    }
}

impl IsType for String {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        (unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32)
    }
}

impl IsType for AnyLuaFunction {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isfunction(ptr, idx) != 0 }
    }
}

impl IsType for AnyNativeFunction {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_iscfunction(ptr, idx) != 0 }
    }
}

impl IsType for LightUserData {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_islightuserdata(ptr, idx) != 0 }
    }
}

impl IsType for Table {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_istable(ptr, idx) != 0 }
    }
}

impl IsType for Coroutine {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isthread(ptr, idx) != 0 }
    }
}

impl IsType for Nil {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isnil(ptr, idx) != 0 }
    }
}

impl IsType for AnyUserData {
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isuserdata(ptr, idx) != 0 }
    }
}
