mod inner_lua;
pub(crate) use inner_lua::InnerLua;

use macros::generate_value_arg_tuple_impl;

use crate::{
    Borrowed, error::UnwrapDisplay, func::FnRef, helper, lstr::StrRef, sys, table::TableRef,
    ud::UdRef,
};
use std::{ffi::CString, fmt::Display, rc::Rc};

use crate::{
    AnyLuaFunction, AnyNativeFunction, AnyUserData, Coroutine, LightUserData, Nil, UserData,
    error::Error, from_lua::FromLua, is_type::IsType, table::Table, to_lua::ToLua,
};

#[derive(Debug)]
pub struct Lua {
    inner: Rc<InnerLua>,
}

impl Lua {
    pub fn new() -> Self {
        let ptr = unsafe { sys::luaL_newstate() };
        if ptr.is_null() {
            panic!("lua out of memory: failed to create state");
        }

        Self {
            inner: InnerLua::new(ptr),
        }
    }

    pub unsafe fn assert_main_state(&self) -> Result<(), Error> {
        let ptr = self.state();
        let is_main = unsafe { sys::lua_pushthread(ptr) == 1 };
        unsafe { sys::lua_pop(ptr, 1) };

        if is_main {
            Ok(())
        } else {
            Err(Error::MainStateNotAvailable)
        }
    }

    pub fn from_ptr(ptr: *mut sys::lua_State) -> Self {
        Self {
            inner: InnerLua::from_ptr(ptr),
        }
    }

    fn state(&self) -> *mut sys::lua_State {
        self.inner.state()
    }

    pub fn open_libs(&self) {
        unsafe { sys::luaL_openlibs(self.state()) };
    }

    pub fn create_table(&self) -> TableRef {
        Table::new(self.inner.clone())
    }

    pub fn create_ref<T: UserData>(&self, value: T) -> UdRef<T> {
        let ptr = self.state();
        <T as ToLua>::to_lua(value, ptr);
        let value = <UdRef<T> as FromLua>::from_lua(ptr, -1).unwrap();
        unsafe { sys::lua_pop(ptr, 1) };
        value
    }

    pub fn create_str(&self, value: &str) -> StrRef {
        StrRef::new(self.inner.clone(), value)
    }

    pub fn register<T: ToLua>(&self, lib_name: &str, lib_instance: T) {
        let ptr = self.state();
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

    pub fn do_file<T: ValueArg + FromLua + ToLua>(&mut self, file_name: &str) -> Result<T, Error> {
        self.eval::<T, _>(|ptr| {
            let file_name = CString::new(file_name)?;
            Ok(unsafe { sys::luaL_loadfile(ptr, file_name.as_ptr() as _) })
        })
    }

    pub fn do_string_with<T: FromLua + ToLua, F: FnOnce(&T) -> R, R>(
        &mut self,
        code: &str,
        f: F,
    ) -> Result<R, Error> {
        self.eval_with::<T, _, _, _>(
            |ptr| {
                let cstring = CString::new(code)?;
                Ok(unsafe { sys::luaL_loadstring(ptr, cstring.as_ptr() as _) })
            },
            f,
        )
    }

    pub fn do_string<T: ValueArg + FromLua + ToLua>(&mut self, code: &str) -> Result<T, Error> {
        self.eval::<T, _>(|ptr| {
            let cstring = CString::new(code)?;
            Ok(unsafe { sys::luaL_loadstring(ptr, cstring.as_ptr() as _) })
        })
    }

    fn eval_with<
        T: FromLua + ToLua,
        F: FnOnce(*mut sys::lua_State) -> Result<std::ffi::c_int, Error>,
        X: FnOnce(&T) -> R,
        R,
    >(
        &mut self,
        f: F,
        x: X,
    ) -> Result<R, Error> {
        let ptr = self.state();
        if f(ptr)? != 0 {
            let msg = <String as FromLua>::from_lua(ptr, -1).unwrap_or_default();
            unsafe { sys::lua_pop(ptr, 1) };
            return Err(Error::InvalidSyntax(msg));
        }

        if unsafe { sys::lua_pcall(ptr, 0, <T as FromLua>::len(), 0) } != 0 {
            unsafe { Err(Error::from_stack(ptr, -1)) }
        } else {
            let size = <T as FromLua>::len();
            let value = T::from_lua(ptr, -size).ok_or(Error::WrongReturnType)?;
            let result = x(&value);
            if size > 0 {
                unsafe { sys::lua_pop(ptr, size) };
            }
            Ok(result)
        }
    }

    fn eval<
        T: ValueArg + FromLua + ToLua,
        F: FnOnce(*mut sys::lua_State) -> Result<std::ffi::c_int, Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<T, Error> {
        let ptr = self.state();
        if f(ptr)? != 0 {
            let msg = <String as FromLua>::from_lua(ptr, -1).unwrap_or_default();
            unsafe { sys::lua_pop(ptr, 1) };
            return Err(Error::InvalidSyntax(msg));
        }

        if unsafe { sys::lua_pcall(ptr, 0, <T as FromLua>::len(), 0) } != 0 {
            unsafe { Err(Error::from_stack(ptr, -1)) }
        } else {
            let size = <T as FromLua>::len();
            let value = T::from_lua(ptr, -size).ok_or(Error::WrongReturnType)?;
            if size > 0 {
                unsafe { sys::lua_pop(ptr, size) };
            }
            Ok(value)
        }
    }

    pub fn try_set_global<T: ToLua>(&mut self, name: &str, value: T) -> Result<(), Error> {
        let ptr = self.state();
        let len = T::len() + 1;
        unsafe {
            helper::try_check_stack(ptr, len)?;
            sys::lua_pushlstring(ptr, name.as_ptr() as _, name.len());
            value.to_lua_unchecked(ptr);
            sys::lua_settable(ptr, sys::LUA_GLOBALSINDEX);
        }
        Ok(())
    }

    pub fn set_global<T: ToLua>(&mut self, name: &str, value: T) {
        self.try_set_global(name, value).unwrap_display();
    }

    pub fn try_get_global<T: FromLua + ValueArg>(&self, name: &str) -> Result<Option<T>, Error> {
        let ptr = self.state();
        unsafe {
            helper::try_check_stack(ptr, 1)?;
            sys::lua_pushlstring(ptr, name.as_ptr() as _, name.len());
            sys::lua_gettable(ptr, sys::LUA_GLOBALSINDEX);
        }

        let out = T::from_lua(ptr, -1);
        unsafe { sys::lua_pop(ptr, 1) };
        Ok(out)
    }

    pub fn get_global<T: FromLua + ValueArg>(&self, name: &str) -> Option<T> {
        self.try_get_global(name).unwrap_display()
    }

    pub fn with_global<T: FromLua, F: FnOnce(&T) -> R, R>(&self, name: &str, f: F) -> Option<R> {
        let ptr = self.state();
        let top = unsafe { sys::lua_gettop(ptr) };

        unsafe {
            sys::lua_pushlstring(ptr, name.as_ptr() as _, name.len());
            sys::lua_gettable(ptr, sys::LUA_GLOBALSINDEX);
        }

        let result = if let Some(value) = T::from_lua(ptr, -1) {
            Some(f(&value))
        } else {
            None
        };

        unsafe { sys::lua_settop(ptr, top) };
        result
    }

    pub fn top(&self) -> i32 {
        unsafe { sys::lua_gettop(self.state()) }
    }
}

impl Display for Lua {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr = self.state();
        let size = self.top();
        writeln!(f, "Stack: {}", size)?;

        for i in 1..=size {
            write!(f, "[{i}/-{}] ", size - i + 1)?;
            if <i32 as IsType>::is_type(ptr, i) {
                writeln!(f, "{}", <i32 as FromLua>::from_lua(ptr, i).unwrap())?;
            } else if <f32 as IsType>::is_type(ptr, i) {
                writeln!(f, "{}", <f32 as FromLua>::from_lua(ptr, i).unwrap())?;
            } else if <f64 as IsType>::is_type(ptr, i) {
                writeln!(f, "{}", <f64 as FromLua>::from_lua(ptr, i).unwrap())?;
            } else if <bool as IsType>::is_type(ptr, i) {
                writeln!(f, "{}", <bool as FromLua>::from_lua(ptr, i).unwrap())?;
            } else if <String as IsType>::is_type(ptr, i) {
                writeln!(f, "{}", <String as FromLua>::from_lua(ptr, i).unwrap())?;
            } else if <AnyLuaFunction as IsType>::is_type(ptr, i) {
                writeln!(f, "function")?;
            } else if <AnyNativeFunction as IsType>::is_type(ptr, i) {
                writeln!(f, "native function")?;
            } else if <LightUserData as IsType>::is_type(ptr, i) {
                writeln!(f, "light user data")?;
            } else if <Table<Borrowed> as IsType>::is_type(ptr, i) {
                writeln!(f, "table")?;
            } else if <Coroutine as IsType>::is_type(ptr, i) {
                writeln!(f, "coroutine")?;
            } else if <Nil as IsType>::is_type(ptr, i) {
                writeln!(f, "nil")?;
            } else if <AnyUserData as IsType>::is_type(ptr, i) {
                writeln!(f, "user data")?;
            }
        }

        Ok(())
    }
}

pub unsafe trait ValueArg {}

macro_rules! impl_value_arg {
    ($($ty:ty),*) => { $(unsafe impl ValueArg for $ty {} )* };
}

impl_value_arg!((), i32, f32, f64, bool, String, StrRef, TableRef, Vec<u8>);

unsafe impl<T> ValueArg for UdRef<T> where T: UserData {}

unsafe impl<T> ValueArg for Option<T> where T: FromLua + ValueArg {}

unsafe impl<I, O> ValueArg for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
}

generate_value_arg_tuple_impl!();

pub fn ensure_value_arg<T: ValueArg>() {}
