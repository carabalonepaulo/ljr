use crate::{lstr::StackStr, lua::ValueArg, sys};
use macros::generate_from_lua_tuple_impl;

pub unsafe trait FromLua: Sized {
    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self>;

    fn len() -> i32 {
        1
    }
}

unsafe impl FromLua for i32 {
    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) } as i32)
        } else {
            None
        }
    }
}

unsafe impl FromLua for f32 {
    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) } as f32)
        } else {
            None
        }
    }
}

unsafe impl FromLua for f64 {
    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_tonumber(ptr, idx) })
        } else {
            None
        }
    }
}

unsafe impl FromLua for bool {
    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_isboolean(ptr, idx) != 0 } {
            Some(unsafe { sys::lua_toboolean(ptr, idx) != 0 })
        } else {
            None
        }
    }
}

unsafe impl FromLua for String {
    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self> {
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

unsafe impl<T> FromLua for Option<T>
where
    T: FromLua + ValueArg,
{
    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TNIL as i32 {
            Some(None)
        } else {
            <T as FromLua>::from_lua(ptr, idx).map(Some)
        }
    }

    fn len() -> i32 {
        T::len()
    }
}

unsafe impl FromLua for () {
    fn from_lua(_: *mut crate::sys::lua_State, _: i32) -> Option<Self> {
        Some(())
    }

    fn len() -> i32 {
        0
    }
}

unsafe impl FromLua for Vec<u8> {
    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self> {
        let temp = StackStr::from_lua(ptr, idx)?;
        Some(temp.as_slice().to_vec())
    }
}

generate_from_lua_tuple_impl!();
