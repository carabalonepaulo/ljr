use crate::{error::Error, lstr::StackStr, lua::ValueArg, sys};
use macros::generate_from_lua_tuple_impl;

pub unsafe trait FromLua: Sized {
    const LEN: i32 = 1;

    fn try_from_lua(ptr: *mut sys::lua_State, idx: i32) -> Result<Self, Error>;

    fn len() -> i32 {
        Self::LEN
    }
}

unsafe impl FromLua for i32 {
    fn try_from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Result<Self, Error> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Ok(unsafe { sys::lua_tonumber(ptr, idx) } as i32)
        } else {
            Err(Error::UnexpectedType)
        }
    }
}

unsafe impl FromLua for f32 {
    fn try_from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Result<Self, Error> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Ok(unsafe { sys::lua_tonumber(ptr, idx) } as f32)
        } else {
            Err(Error::UnexpectedType)
        }
    }
}

unsafe impl FromLua for f64 {
    fn try_from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Result<Self, Error> {
        if unsafe { sys::lua_isnumber(ptr, idx) != 0 } {
            Ok(unsafe { sys::lua_tonumber(ptr, idx) })
        } else {
            Err(Error::UnexpectedType)
        }
    }
}

unsafe impl FromLua for bool {
    fn try_from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Result<Self, Error> {
        if unsafe { sys::lua_isboolean(ptr, idx) != 0 } {
            Ok(unsafe { sys::lua_toboolean(ptr, idx) != 0 })
        } else {
            Err(Error::UnexpectedType)
        }
    }
}

unsafe impl FromLua for String {
    fn try_from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Result<Self, Error> {
        unsafe {
            if sys::lua_type(ptr, idx) == sys::LUA_TSTRING as i32 {
                let mut len = 0;
                let char_ptr = sys::lua_tolstring(ptr, idx, &mut len);
                if char_ptr.is_null() {
                    return Err(Error::InvalidCString);
                }

                let slice = std::slice::from_raw_parts(char_ptr as *const u8, len);
                Ok(std::str::from_utf8(slice)?.into())
            } else {
                Err(Error::UnexpectedType)
            }
        }
    }
}

unsafe impl<T> FromLua for Option<T>
where
    T: FromLua + ValueArg,
{
    const LEN: i32 = T::LEN;

    fn try_from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Result<Self, Error> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TNIL as i32 {
            Ok(None)
        } else {
            <T as FromLua>::try_from_lua(ptr, idx).map(Some)
        }
    }
}

unsafe impl FromLua for () {
    const LEN: i32 = 0;

    fn try_from_lua(_: *mut crate::sys::lua_State, _: i32) -> Result<Self, Error> {
        Ok(())
    }
}

unsafe impl FromLua for Vec<u8> {
    fn try_from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Result<Self, Error> {
        let temp = StackStr::try_from_lua(ptr, idx)?;
        Ok(temp.as_slice().to_vec())
    }
}

generate_from_lua_tuple_impl!();
