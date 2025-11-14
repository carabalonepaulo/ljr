use crate::UserData;

use luajit2_sys as sys;

macro_rules! lua_error {
    ($ptr:ident, $msg:expr) => {{
        let c_err_msg = std::ffi::CString::new($msg).unwrap();
        unsafe {
            sys::lua_pushstring($ptr, c_err_msg.as_ptr());
            sys::lua_error($ptr);
        }
        unreachable!()
    }};
}

pub fn from_lua<T: crate::from_lua::FromLua>(
    ptr: *mut sys::lua_State,
    idx: &mut i32,
    expected_type: &str,
) -> T::Output {
    match <T as crate::from_lua::FromLua>::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += T::len();
            value
        }
        None => {
            let msg = format!("invalid argument {}, expected {}", idx, expected_type);
            lua_error!(ptr, msg);
        }
    }
}

pub fn from_lua_ref<T>(ptr: *mut sys::lua_State, idx: &mut i32) -> crate::lua_ref::LuaRef<T>
where
    T: UserData,
{
    match <T as crate::from_lua::FromLua>::from_lua(ptr, *idx) {
        Some(value) => {
            *idx += <T as crate::from_lua::FromLua>::len();
            value
        }
        None => {
            let msg = format!("invalid argument {}, expected {}", idx, unsafe {
                std::ffi::CStr::from_ptr(T::name()).to_str().unwrap()
            });
            lua_error!(ptr, msg);
        }
    }
}

pub fn catch<F, R>(ptr: *mut sys::lua_State, f: F) -> std::ffi::c_int
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
    R: crate::to_lua::ToLua,
{
    let result = std::panic::catch_unwind(f);
    match result {
        Ok(value) => {
            crate::to_lua::ToLua::to_lua(value, ptr);
            <R as crate::to_lua::ToLua>::len() as _
        }
        Err(e) => {
            let err_msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };

            let c_err_msg = format!("Rust panic: {}", err_msg);
            unsafe {
                sys::lua_pushlstring(ptr, c_err_msg.as_ptr() as _, c_err_msg.len());
                sys::lua_error(ptr);
            }

            unreachable!()
        }
    }
}
