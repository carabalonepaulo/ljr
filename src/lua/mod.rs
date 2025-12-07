mod inner_lua;
pub(crate) use inner_lua::InnerLua;

use macros::generate_value_arg_tuple_impl;

use crate::{
    Borrowed,
    error::UnwrapDisplay,
    func::FnRef,
    helper,
    lstr::StrRef,
    prelude::TableView,
    stack_guard::StackGuard,
    sys,
    table::{StackTable, TableRef},
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
    pub fn try_new() -> Result<Self, Error> {
        let ptr = unsafe { sys::luaL_newstate() };
        if ptr.is_null() {
            Err(Error::StateAllocationFailed)
        } else {
            Ok(Self {
                inner: InnerLua::new(ptr),
            })
        }
    }

    pub fn new() -> Self {
        Self::try_new().unwrap_display()
    }

    pub unsafe fn assert_main_state(&self) -> Result<(), Error> {
        let ptr = self.inner.try_state()?;
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

    fn use_globals<F: FnOnce(&mut StackTable) -> R, R>(&self, f: F) -> Result<R, Error> {
        let ptr = self.inner.try_state()?;
        unsafe {
            helper::try_check_stack(ptr, 1)?;
            sys::lua_pushvalue(ptr, sys::LUA_GLOBALSINDEX);
            let mut table = StackTable::from_stack(ptr, -1);
            let result = f(&mut table);
            sys::lua_pop(ptr, 1);
            Ok(result)
        }
    }

    pub fn try_with_globals<F: FnOnce(&TableView) -> R, R>(&self, f: F) -> Result<R, Error> {
        self.use_globals(|t| {
            let guard = t.as_ref();
            f(&*guard)
        })
    }

    pub fn with_globals<F: FnOnce(&TableView) -> R, R>(&self, f: F) -> R {
        self.try_with_globals(f).unwrap_display()
    }

    pub fn try_with_globals_mut<F: FnOnce(&mut TableView) -> R, R>(
        &mut self,
        f: F,
    ) -> Result<R, Error> {
        self.use_globals(|t| {
            let mut guard = t.as_mut();
            f(&mut *guard)
        })
    }

    pub fn with_globals_mut<F: FnOnce(&mut TableView) -> R, R>(&mut self, f: F) -> R {
        self.try_with_globals_mut(f).unwrap_display()
    }

    pub fn try_globals(&self) -> Result<TableRef, Error> {
        let ptr = self.inner.try_state()?;
        unsafe {
            helper::try_check_stack(ptr, 2)?;
            sys::lua_pushvalue(ptr, sys::LUA_GLOBALSINDEX);
            let table = TableRef::from_stack(ptr, -1);
            sys::lua_pop(ptr, 1);
            Ok(table)
        }
    }

    pub fn globals(&self) -> TableRef {
        self.try_globals().unwrap_display()
    }

    pub fn try_open_libs(&self) -> Result<(), Error> {
        unsafe { sys::luaL_openlibs(self.inner.try_state()?) };
        Ok(())
    }

    pub fn open_libs(&self) {
        self.try_open_libs().unwrap_display()
    }

    pub fn try_create_table(&self) -> Result<TableRef, Error> {
        Table::try_new(self.inner.clone())
    }

    pub fn create_table(&self) -> TableRef {
        Table::new(self.inner.clone())
    }

    pub fn try_create_ref<T: UserData>(&self, value: T) -> Result<UdRef<T>, Error> {
        let ptr = self.inner.try_state()?;
        <T as ToLua>::try_to_lua(value, ptr)?;
        let value = <UdRef<T> as FromLua>::try_from_lua(ptr, -1)?;
        unsafe { sys::lua_pop(ptr, 1) };
        Ok(value)
    }

    pub fn create_ref<T: UserData>(&self, value: T) -> UdRef<T> {
        self.try_create_ref(value).unwrap_display()
    }

    pub fn try_create_str(&self, value: &str) -> Result<StrRef, Error> {
        StrRef::try_new(self.inner.clone(), value)
    }

    pub fn create_str(&self, value: &str) -> StrRef {
        StrRef::new(self.inner.clone(), value)
    }

    pub fn try_register<T: ToLua>(&self, lib_name: &str, lib_instance: T) -> Result<(), Error> {
        let ptr = self.inner.try_state()?;
        let cname = std::ffi::CString::new(lib_name)?;

        unsafe {
            helper::try_check_stack(ptr, 3)?;

            sys::lua_getglobal(ptr, c"package".as_ptr());
            sys::lua_getfield(ptr, -1, c"loaded".as_ptr());

            lib_instance.to_lua_unchecked(ptr);
            sys::lua_setfield(ptr, -2, cname.as_ptr());

            sys::lua_pop(ptr, 2);
        }

        Ok(())
    }

    pub fn register<T: ToLua>(&self, lib_name: &str, lib_instance: T) {
        self.try_register(lib_name, lib_instance).unwrap_display()
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
        let ptr = self.inner.try_state()?;
        let _g = StackGuard::new(ptr);

        if f(ptr)? != 0 {
            let msg = <String as FromLua>::try_from_lua(ptr, -1).unwrap_or_default();
            return Err(Error::InvalidSyntax(msg));
        }

        if unsafe { sys::lua_pcall(ptr, 0, <T as FromLua>::len(), 0) } != 0 {
            unsafe { Err(Error::from_stack(ptr, -1)) }
        } else {
            let size = <T as FromLua>::len();
            let value = T::try_from_lua(ptr, -size).map_err(|_| Error::WrongReturnType)?;
            let result = x(&value);
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
        let ptr = self.inner.try_state()?;
        let _g = StackGuard::new(ptr);

        if f(ptr)? != 0 {
            let msg = <String as FromLua>::try_from_lua(ptr, -1)?;
            return Err(Error::InvalidSyntax(msg));
        }

        if unsafe { sys::lua_pcall(ptr, 0, <T as FromLua>::len(), 0) } != 0 {
            unsafe { Err(Error::from_stack(ptr, -1)) }
        } else {
            let size = <T as FromLua>::len();
            let value = T::try_from_lua(ptr, -size).map_err(|_| Error::WrongReturnType)?;
            Ok(value)
        }
    }

    pub fn try_top(&self) -> Result<i32, Error> {
        Ok(unsafe { sys::lua_gettop(self.inner.try_state()?) })
    }

    pub fn top(&self) -> i32 {
        self.try_top().unwrap_display()
    }
}

impl Display for Lua {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr = self.inner.try_state().unwrap_display();
        let size = self.top();
        writeln!(f, "Stack: {}", size)?;

        for i in 1..=size {
            write!(f, "[{i}/-{}] ", size - i + 1)?;
            if <i32 as IsType>::is_type(ptr, i) {
                writeln!(
                    f,
                    "{}",
                    <i32 as FromLua>::try_from_lua(ptr, i).unwrap_display()
                )?;
            } else if <f32 as IsType>::is_type(ptr, i) {
                writeln!(
                    f,
                    "{}",
                    <f32 as FromLua>::try_from_lua(ptr, i).unwrap_display()
                )?;
            } else if <f64 as IsType>::is_type(ptr, i) {
                writeln!(
                    f,
                    "{}",
                    <f64 as FromLua>::try_from_lua(ptr, i).unwrap_display()
                )?;
            } else if <bool as IsType>::is_type(ptr, i) {
                writeln!(
                    f,
                    "{}",
                    <bool as FromLua>::try_from_lua(ptr, i).unwrap_display()
                )?;
            } else if <String as IsType>::is_type(ptr, i) {
                writeln!(
                    f,
                    "{}",
                    <String as FromLua>::try_from_lua(ptr, i).unwrap_display()
                )?;
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
