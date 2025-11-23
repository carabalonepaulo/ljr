use crate::sys;
use macros::generate_from_lua_tuple_impl;

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
                let mut len = 0;
                let char_ptr = sys::lua_tolstring(ptr, idx, &mut len);
                if char_ptr.is_null() {
                    return None;
                }

                let slice = std::slice::from_raw_parts(char_ptr as *const u8, len);
                std::str::from_utf8(slice).ok().map(|s| s.to_string())
            } else {
                None
            }
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
