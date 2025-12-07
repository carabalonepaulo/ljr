use std::panic::AssertUnwindSafe;

use crate::Nil;
use crate::UserData;

use crate::error::Error;
use crate::from_lua::FromLua;
use crate::is_type::IsType;
use crate::lstr::StackStr;
use crate::sys;
use crate::ud::StackUd;

fn user_data_unique_name<T: UserData>() -> String {
    unsafe {
        let s = std::ffi::CStr::from_ptr(T::name());
        s.to_string_lossy().to_string()
    }
}

fn raise_error(ptr: *mut sys::lua_State, msg: String) -> ! {
    unsafe {
        if sys::lua_checkstack(ptr, 1) == 0 {
            sys::lua_pop(ptr, 1);
        }
        sys::lua_pushlstring_(ptr, msg.as_ptr() as _, msg.len());
        std::mem::drop(msg);
    }
    unsafe { sys::lua_error(ptr) };
}

pub fn check_arg_count(ptr: *mut sys::lua_State, expected: usize) -> Result<(), Error> {
    let got = unsafe { crate::sys::lua_gettop(ptr) } as usize;
    if got == expected {
        Ok(())
    } else {
        Err(Error::ArgumentCountMismatch(expected, got))
    }
}

pub fn from_lua<T: crate::from_lua::FromLua>(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
    expected_type: &str,
) -> Result<T, Error> {
    match <T as crate::from_lua::FromLua>::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += T::len();
            Ok(value)
        }
        None => Err(Error::ArgumentTypeMismatch(*idx as _, expected_type.into())),
    }
}

pub fn from_lua_opt<T: FromLua>(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
) -> Result<Option<T>, Error> {
    match T::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += T::len();
            Ok(Some(value))
        }
        None => {
            if Nil::is_type(ptr, *idx) {
                Ok(None)
            } else {
                Err(Error::ArgumentTypeMismatch(
                    *idx as _,
                    "value or nil".into(),
                ))
            }
        }
    }
}

pub fn from_lua_opt_str(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
) -> Result<Option<StackStr>, Error> {
    match StackStr::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += StackStr::len();
            Ok(Some(value))
        }
        None => {
            if Nil::is_type(ptr, *idx) {
                Ok(None)
            } else {
                Err(Error::ArgumentTypeMismatch(*idx as _, "&str or nil".into()))
            }
        }
    }
}

pub fn from_lua_opt_stack_ud<T>(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
) -> Result<Option<StackUd<T>>, Error>
where
    T: UserData,
{
    match <StackUd<T> as crate::from_lua::FromLua>::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += <StackUd<T> as crate::from_lua::FromLua>::len();
            Ok(Some(value))
        }
        None => {
            if Nil::is_type(ptr, *idx) {
                Ok(None)
            } else {
                Err(Error::ArgumentTypeMismatch(
                    *idx as _,
                    format!("{} or nil", user_data_unique_name::<T>()),
                ))
            }
        }
    }
}

pub fn from_lua_stack_ref<T>(ptr: *mut sys::lua_State, idx: &mut i32) -> Result<StackUd<T>, Error>
where
    T: UserData,
{
    match <StackUd<T> as crate::from_lua::FromLua>::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += <StackUd<T> as crate::from_lua::FromLua>::len();
            Ok(value)
        }
        None => Err(Error::ArgumentTypeMismatch(
            *idx as _,
            format!("{} or nil", user_data_unique_name::<T>()),
        )),
    }
}

pub fn catch<F, R>(ptr: *mut sys::lua_State, f: F) -> std::ffi::c_int
where
    F: FnOnce() -> Result<R, Error>,
    R: crate::to_lua::ToLua,
{
    let result: Result<std::ffi::c_int, String> = {
        let result = std::panic::catch_unwind(AssertUnwindSafe(f));
        match result {
            Ok(r) => match r {
                Ok(r) => {
                    crate::to_lua::ToLua::to_lua(r, ptr);
                    Ok(<R as crate::to_lua::ToLua>::len() as _)
                }
                Err(msg) => Err(msg.to_string()),
            },
            Err(e) => {
                let msg = {
                    let err_msg = if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "unknown error".to_string()
                    };
                    format!("Rust panic: {}", err_msg)
                };
                Err(msg)
            }
        }
    };

    match result {
        Ok(n) => n,
        Err(msg) => raise_error(ptr, msg),
    }
}

#[inline(always)]
pub(crate) unsafe fn try_check_stack(ptr: *mut sys::lua_State, len: i32) -> Result<(), Error> {
    unsafe {
        if sys::lua_checkstack(ptr, len) == 0 {
            Err(Error::StackCapacityExceeded)
        } else {
            Ok(())
        }
    }
}
