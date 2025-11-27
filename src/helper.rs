use std::panic::AssertUnwindSafe;

use crate::Nil;
use crate::UserData;

use crate::from_lua::FromLua;
use crate::is_type::IsType;
use crate::lstr::StackStr;
use crate::sys;
use crate::ud::StackUd;

fn raise_error(ptr: *mut sys::lua_State, msg: String) -> ! {
    unsafe {
        sys::lua_pushlstring_(ptr, msg.as_ptr() as _, msg.len());
        std::mem::drop(msg);
    }
    unsafe { sys::lua_error(ptr) };
}

pub fn check_arg_count(ptr: *mut sys::lua_State, expected: usize) -> Result<(), String> {
    let got = unsafe { crate::sys::lua_gettop(ptr) } as usize;
    if got == expected {
        Ok(())
    } else {
        Err(format!(
            "wrong number of arguments, expecting {}, got {}",
            expected, got
        ))
    }
}

pub fn from_lua<T: crate::from_lua::FromLua>(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
    expected_type: &str,
) -> Result<T, String> {
    match <T as crate::from_lua::FromLua>::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += T::len();
            Ok(value)
        }
        None => Err(format!(
            "invalid argument {}, expected {}",
            idx, expected_type
        )),
    }
}

pub fn from_lua_opt<T: FromLua>(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
) -> Result<Option<T>, String> {
    match T::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += T::len();
            Ok(Some(value))
        }
        None => {
            if Nil::is_type(ptr, *idx) {
                Ok(None)
            } else {
                Err(format!("invalid argument {}, expected &str or nil", idx))
            }
        }
    }
}

pub fn from_lua_opt_str(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
) -> Result<Option<StackStr>, String> {
    match StackStr::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += StackStr::len();
            Ok(Some(value))
        }
        None => {
            if Nil::is_type(ptr, *idx) {
                Ok(None)
            } else {
                Err(format!("invalid argument {}, expected &str or nil", idx))
            }
        }
    }
}

pub fn from_lua_opt_stack_ud<T>(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
) -> Result<Option<StackUd<T>>, String>
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
                Err(format!(
                    "invalid argument {}, expected {} or nil",
                    idx,
                    unsafe { std::ffi::CStr::from_ptr(T::name()).to_str().unwrap() }
                ))
            }
        }
    }
}

pub fn from_lua_stack_ref<T>(ptr: *mut sys::lua_State, idx: &mut i32) -> Result<StackUd<T>, String>
where
    T: UserData,
{
    match <StackUd<T> as crate::from_lua::FromLua>::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += <StackUd<T> as crate::from_lua::FromLua>::len();
            Ok(value)
        }
        None => Err(format!("invalid argument {}, expected {}", idx, unsafe {
            std::ffi::CStr::from_ptr(T::name()).to_str().unwrap()
        })),
    }
}

pub fn catch<F, R>(ptr: *mut sys::lua_State, f: F) -> std::ffi::c_int
where
    F: FnOnce() -> Result<R, String>,
    R: crate::to_lua::ToLua,
{
    let result = std::panic::catch_unwind(AssertUnwindSafe(f));
    match result {
        Ok(r) => match r {
            Ok(r) => {
                crate::to_lua::ToLua::to_lua(r, ptr);
                <R as crate::to_lua::ToLua>::len() as _
            }
            Err(msg) => raise_error(ptr, msg),
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
            raise_error(ptr, msg)
        }
    }
}
