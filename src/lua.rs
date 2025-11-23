use macros::generate_get_global_tuple_impl;

use crate::{Borrowed, func::FnRef, lstr::StrRef, sys, table::TableRef, ud::Ud};
use std::{ffi::CString, fmt::Display};

use crate::{
    AnyLuaFunction, AnyNativeFunction, AnyUserData, Coroutine, LightUserData, Nil, UserData,
    error::Error, from_lua::FromLua, is_type::IsType, table::Table, to_lua::ToLua,
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

    pub fn create_table(&self) -> TableRef {
        Table::new(self.0)
    }

    pub fn create_ref<T: UserData>(&self, value: T) -> Ud<T> {
        <T as ToLua>::to_lua(value, self.0);
        let ud = Ud::owned(self.0, -1);
        unsafe { sys::lua_pop(self.0, 1) };
        ud
    }

    pub fn create_str(&self, value: &str) -> StrRef {
        StrRef::new(self.0, value)
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

    pub fn exec(&mut self, code: &str) -> Result<(), Error> {
        self.do_string::<()>(code)
    }

    pub fn exec_file(&mut self, code: &str) -> Result<(), Error> {
        self.do_file::<()>(code)
    }

    pub fn do_file<T: GetGlobal + ToLua>(&mut self, file_name: &str) -> Result<T::Output, Error> {
        self.eval::<T, _>(|ptr| {
            let file_name = CString::new(file_name)?;
            Ok(unsafe { sys::luaL_loadfile(ptr, file_name.as_ptr() as _) })
        })
    }

    pub fn do_string<T: GetGlobal + ToLua>(&mut self, code: &str) -> Result<T::Output, Error> {
        self.eval::<T, _>(|ptr| {
            let cstring = CString::new(code)?;
            Ok(unsafe { sys::luaL_loadstring(ptr, cstring.as_ptr() as _) })
        })
    }

    fn eval<
        T: GetGlobal + ToLua,
        F: FnOnce(*mut sys::lua_State) -> Result<std::ffi::c_int, Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<T::Output, Error> {
        if f(self.0)? != 0 {
            let msg = <String as FromLua>::from_lua(self.0, -1).unwrap_or_default();
            unsafe { sys::lua_pop(self.0, 1) };
            return Err(Error::InvalidSyntax(msg));
        }

        if unsafe { sys::lua_pcall(self.0, 0, <T as ToLua>::len(), 0) } != 0 {
            if let Some(msg) = <String as FromLua>::from_lua(self.0, -1) {
                unsafe { sys::lua_pop(self.0, 1) };
                return Err(Error::LuaError(msg));
            } else {
                unsafe { sys::lua_pop(self.0, 1) };
                return Err(Error::UnknownLuaError);
            }
        } else {
            let size = <T as FromLua>::len();
            let value = T::from_lua(self.0, -size).ok_or(Error::WrongReturnType)?;
            if size > 0 {
                unsafe { sys::lua_pop(self.0, size) };
            }
            Ok(value)
        }
    }

    pub fn set_global(&mut self, name: &str, value: impl ToLua) {
        let ptr = self.0;
        unsafe { sys::lua_pushlstring(ptr, name.as_ptr() as _, name.len()) };
        value.to_lua(ptr);
        unsafe { sys::lua_settable(ptr, sys::LUA_GLOBALSINDEX) };
    }

    pub fn get_global<T: GetGlobal>(&self, name: &str) -> Option<T::Output> {
        let ptr = self.0;
        unsafe {
            sys::lua_pushlstring(ptr, name.as_ptr() as _, name.len());
            sys::lua_gettable(ptr, sys::LUA_GLOBALSINDEX);
        }

        let out = T::from_lua(ptr, -1);
        unsafe { sys::lua_pop(ptr, T::len()) };
        out
    }

    pub fn top(&self) -> i32 {
        unsafe { sys::lua_gettop(self.0) }
    }
}

impl Drop for Lua {
    fn drop(&mut self) {
        if self.1 {
            unsafe { sys::lua_close(self.0) };
        }
    }
}

impl Display for Lua {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self.top();
        writeln!(f, "Stack: {}", size)?;

        for i in 1..=size {
            write!(f, "[{i}/-{}] ", size - i + 1)?;
            if <i32 as IsType>::is_type(self.0, i) {
                writeln!(f, "{}", <i32 as FromLua>::from_lua(self.0, i).unwrap())?;
            } else if <f32 as IsType>::is_type(self.0, i) {
                writeln!(f, "{}", <f32 as FromLua>::from_lua(self.0, i).unwrap())?;
            } else if <f64 as IsType>::is_type(self.0, i) {
                writeln!(f, "{}", <f64 as FromLua>::from_lua(self.0, i).unwrap())?;
            } else if <bool as IsType>::is_type(self.0, i) {
                writeln!(f, "{}", <bool as FromLua>::from_lua(self.0, i).unwrap())?;
            } else if <String as IsType>::is_type(self.0, i) {
                writeln!(f, "{}", <String as FromLua>::from_lua(self.0, i).unwrap())?;
            } else if <AnyLuaFunction as IsType>::is_type(self.0, i) {
                writeln!(f, "function")?;
            } else if <AnyNativeFunction as IsType>::is_type(self.0, i) {
                writeln!(f, "native function")?;
            } else if <LightUserData as IsType>::is_type(self.0, i) {
                writeln!(f, "light user data")?;
            } else if <Table<Borrowed> as IsType>::is_type(self.0, i) {
                writeln!(f, "table")?;
            } else if <Coroutine as IsType>::is_type(self.0, i) {
                writeln!(f, "coroutine")?;
            } else if <Nil as IsType>::is_type(self.0, i) {
                writeln!(f, "nil")?;
            } else if <AnyUserData as IsType>::is_type(self.0, i) {
                writeln!(f, "user data")?;
            }
        }

        Ok(())
    }
}

pub trait GetGlobal: FromLua {}

macro_rules! impl_get_global {
    ($($ty:ty),*) => { $(impl GetGlobal for $ty {} )* };
}

impl_get_global!((), i32, f32, f64, bool, String, StrRef, TableRef);

impl<T> GetGlobal for Ud<T> where T: UserData {}

impl<T> GetGlobal for Option<T>
where
    T: FromLua,
    T::Output: FromLua,
{
}

impl<I, O> GetGlobal for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
}

generate_get_global_tuple_impl!();
