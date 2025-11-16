use luajit2_sys as sys;
use std::ffi::CString;

use crate::{
    UserData, error::Error, from_lua::FromLua, lua_ref::LuaRef, stack::Stack, table::Table,
    to_lua::ToLua,
};

#[derive(Debug)]
pub struct Lua(*mut sys::lua_State, bool);

impl Lua {
    pub fn new() -> Self {
        let ptr = unsafe { sys::luaL_newstate() };
        Self(ptr, true)
    }

    pub fn from_ptr(ptr: *mut sys::lua_State) -> Self {
        Self(ptr, false)
    }

    pub fn open_libs(&self) {
        unsafe { sys::luaL_openlibs(self.0) };
    }

    pub fn create_table(&self) -> Table {
        Table::new(self.0)
    }

    pub fn create_ref<T: UserData>(&self, value: T) -> LuaRef<T> {
        value.to_lua(self.0);
        LuaRef::new(self.0)
    }

    pub fn register<T: ToLua>(&self, lib_name: &str, lib_instance: T) {
        let ptr = self.0;
        let cname = std::ffi::CString::new(lib_name).unwrap();

        unsafe {
            sys::lua_getglobal(ptr, c"package".as_ptr());
            sys::lua_getfield(ptr, -1, c"loaded".as_ptr());

            lib_instance.to_lua(ptr);
            sys::lua_setfield(ptr, -2, cname.as_ptr());

            sys::lua_pop(ptr, 2);
        }
    }

    pub(crate) fn owned(&self) -> bool {
        self.1
    }

    pub fn stack(&self) -> Stack {
        Stack(self.0)
    }

    pub fn do_string<T: FromLua>(&mut self, code: &str) -> Result<T::Output, Error> {
        let old_top = unsafe { sys::lua_gettop(self.0) };
        let cstring = CString::new(code)?;
        if unsafe { sys::luaL_loadstring(self.0, cstring.as_ptr() as _) } != 0 {
            let msg = <String as FromLua>::from_lua(self.0, -1).unwrap_or_default();
            unsafe { sys::lua_pop(self.0, 1) };
            return Err(Error::InvalidSyntax(msg));
        }

        if unsafe { sys::lua_pcall(self.0, 0, sys::LUA_MULTRET, 0) } != 0 {
            if let Some(msg) = <String as FromLua>::from_lua(self.0, -1) {
                unsafe { sys::lua_pop(self.0, 1) };
                return Err(Error::LuaError(msg));
            } else {
                unsafe { sys::lua_pop(self.0, 1) };
                return Err(Error::UnknownLuaError);
            }
        } else {
            let diff = unsafe { sys::lua_gettop(self.0) } - old_top;
            let size = T::len();
            if diff != size {
                return Err(Error::WrongReturnType);
            }

            let value = T::from_lua(self.0, -size).ok_or(Error::WrongReturnType)?;
            unsafe { sys::lua_pop(self.0, size) };
            Ok(value)
        }
    }

    pub fn set_global(&mut self, name: &str, value: impl ToLua) {
        self.stack().push(value);
        let str = CString::new(name).unwrap();
        unsafe { sys::lua_setglobal(self.0, str.as_ptr()) };
    }
}

impl Drop for Lua {
    fn drop(&mut self) {
        if self.owned() {
            unsafe { sys::lua_close(self.0) };
        }
    }
}
