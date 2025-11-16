use std::ffi::CStr;

use luajit2_sys as sys;

use crate::{UserData, lua_ref::LuaRef, stack_ref::StackRef, stack_str::StackStr, table::Table};

pub trait FromLua {
    type Output;

    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self::Output>;

    fn len() -> i32 {
        1
    }
}

impl FromLua for i32 {
    type Output = i32;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) } as i32)
        } else {
            None
        }
    }
}

impl FromLua for f32 {
    type Output = f32;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) } as f32)
        } else {
            None
        }
    }
}

impl FromLua for f64 {
    type Output = f64;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) })
        } else {
            None
        }
    }
}

impl FromLua for bool {
    type Output = bool;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isboolean(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_toboolean(ptr, idx) != 0 })
        } else {
            None
        }
    }
}

impl FromLua for String {
    type Output = String;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
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

impl<T> FromLua for T
where
    T: UserData,
{
    type Output = StackRef<T>;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        unsafe {
            sys::lua_pushvalue(ptr, idx);

            if sys::lua_getmetatable(ptr, -1) == 0 {
                sys::lua_pop(ptr, 1);
                return None;
            }

            sys::lua_getfield(ptr, -1, c"__name".as_ptr());
            let mt_name = sys::lua_tostring(ptr, -1);

            let mt = CStr::from_ptr(mt_name);
            let expected = CStr::from_ptr(T::name());
            if mt != expected {
                sys::lua_pop(ptr, 3);
                return None;
            }

            sys::lua_pop(ptr, 2);
        }
        Some(StackRef::new(ptr, idx))
    }
}

impl<T: UserData> FromLua for LuaRef<T> {
    type Output = LuaRef<T>;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        unsafe {
            sys::lua_pushvalue(ptr, idx);

            if sys::lua_getmetatable(ptr, -1) == 0 {
                sys::lua_pop(ptr, 1);
                return None;
            }

            sys::lua_getfield(ptr, -1, b"__name\0".as_ptr() as _);
            let mt_name_ptr = sys::lua_tostring(ptr, -1);
            if mt_name_ptr.is_null() {
                sys::lua_pop(ptr, 2);
                return None;
            }

            let mt_name = CStr::from_ptr(mt_name_ptr);
            let expected_name = CStr::from_ptr(T::name());
            if mt_name != expected_name {
                sys::lua_pop(ptr, 2);
                return None;
            }

            sys::lua_pop(ptr, 2);

            Some(LuaRef::new(ptr))
        }
    }
}

impl FromLua for Table {
    type Output = Table;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_istable(ptr, idx) } != 0 {
            Some(Table::from_stack(ptr, idx))
        } else {
            None
        }
    }
}

impl FromLua for StackStr {
    type Output = StackStr;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            StackStr::new(ptr, idx).ok()
        } else {
            None
        }
    }
}

impl FromLua for () {
    type Output = ();

    fn from_lua(_: *mut luajit2_sys::lua_State, _: i32) -> Option<Self::Output> {
        Some(())
    }
}
