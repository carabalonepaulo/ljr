use macros::generate_get_global_tuple_impl;

use crate::{Borrowed, func::FnRef, lstr::StrRef, sys, table::TableRef, ud::Ud};
use std::{cell::Cell, ffi::CString, fmt::Display, ptr, rc::Rc};

use crate::{
    AnyLuaFunction, AnyNativeFunction, AnyUserData, Coroutine, LightUserData, Nil, UserData,
    error::Error, from_lua::FromLua, is_type::IsType, table::Table, to_lua::ToLua,
};

static CTX_KEY: u8 = 0;

unsafe extern "C-unwind" fn sentinel_gc(ptr: *mut sys::lua_State) -> std::ffi::c_int {
    unsafe {
        let ctx_key = &CTX_KEY as *const u8 as *mut std::ffi::c_void;
        sys::lua_pushlightuserdata(ptr, ctx_key);
        sys::lua_gettable(ptr, sys::LUA_REGISTRYINDEX);

        let weak_ptr = sys::lua_touserdata(ptr, -1) as *mut std::rc::Weak<InnerLua>;
        sys::lua_pop(ptr, 1);

        if !weak_ptr.is_null() {
            let weak = Box::from_raw(weak_ptr);
            if let Some(inner) = weak.upgrade() {
                inner.state.set(ptr::null_mut());
            }
        }
    }
    0
}

#[derive(Debug)]
pub struct InnerLua {
    state: Cell<*mut sys::lua_State>,
    owned: bool,
}

impl InnerLua {
    unsafe fn attach_sentinel(ptr: *mut sys::lua_State, inner: Rc<InnerLua>) {
        let ctx_key = &CTX_KEY as *const u8 as *mut std::ffi::c_void;
        let weak = Rc::downgrade(&inner);
        let weak_ptr = Box::into_raw(Box::new(weak));

        unsafe {
            sys::lua_pushlightuserdata(ptr, ctx_key);
            sys::lua_pushlightuserdata(ptr, weak_ptr as *mut _);
            sys::lua_settable(ptr, sys::LUA_REGISTRYINDEX);

            let _ = sys::lua_newuserdata(ptr, 0);
            sys::lua_newtable(ptr);
            sys::lua_pushstring(ptr, c"__gc".as_ptr());
            sys::lua_pushcfunction(ptr, sentinel_gc);
            sys::lua_settable(ptr, -3);
            sys::lua_setmetatable(ptr, -2);

            sys::lua_setfield(ptr, sys::LUA_REGISTRYINDEX, c"__LJR_SENTINEL".as_ptr());
        }
    }

    pub(crate) fn from_ptr(ptr: *mut sys::lua_State) -> Rc<Self> {
        unsafe {
            let ctx_key = &CTX_KEY as *const u8 as *mut std::ffi::c_void;

            sys::lua_pushlightuserdata(ptr, ctx_key);
            sys::lua_gettable(ptr, sys::LUA_REGISTRYINDEX);

            let data_ptr = sys::lua_touserdata(ptr, -1);
            sys::lua_pop(ptr, 1);

            if !data_ptr.is_null() {
                let weak_ptr = data_ptr as *mut std::rc::Weak<InnerLua>;
                let weak = &*weak_ptr;
                if let Some(rc) = weak.upgrade() {
                    return rc;
                }
            }

            let inner = Rc::new(InnerLua {
                state: Cell::new(ptr),
                owned: false,
            });

            Self::attach_sentinel(ptr, inner.clone());

            inner
        }
    }

    pub(crate) fn state(&self) -> *mut sys::lua_State {
        let ptr = self.state.get();
        if ptr.is_null() {
            panic!("lua state has been closed");
        }
        ptr
    }

    pub(crate) fn state_or_null(&self) -> *mut sys::lua_State {
        self.state.get()
    }
}

impl Drop for InnerLua {
    fn drop(&mut self) {
        let ptr = self.state.get();
        if !ptr.is_null() && self.owned {
            unsafe { sys::lua_close(ptr) };
        }
    }
}

#[derive(Debug)]
pub struct Lua {
    inner: Rc<InnerLua>,
}

impl Lua {
    pub fn new() -> Self {
        let ptr = unsafe { sys::luaL_newstate() };
        let inner = Rc::new(InnerLua {
            state: Cell::new(ptr),
            owned: true,
        });
        unsafe { InnerLua::attach_sentinel(ptr, inner.clone()) };
        Self { inner }
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

    pub fn create_ref<T: UserData>(&self, value: T) -> Ud<T> {
        let ptr = self.state();
        <T as ToLua>::to_lua(value, ptr);
        let ud = Ud::owned(self.inner.clone(), -1);
        unsafe { sys::lua_pop(ptr, 1) };
        ud
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

    pub fn do_file<T: ValueArg + ToLua>(&mut self, file_name: &str) -> Result<T::Output, Error> {
        self.eval::<T, _>(|ptr| {
            let file_name = CString::new(file_name)?;
            Ok(unsafe { sys::luaL_loadfile(ptr, file_name.as_ptr() as _) })
        })
    }

    pub fn do_string<T: ValueArg + ToLua>(&mut self, code: &str) -> Result<T::Output, Error> {
        self.eval::<T, _>(|ptr| {
            let cstring = CString::new(code)?;
            Ok(unsafe { sys::luaL_loadstring(ptr, cstring.as_ptr() as _) })
        })
    }

    fn eval<
        T: ValueArg + ToLua,
        F: FnOnce(*mut sys::lua_State) -> Result<std::ffi::c_int, Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<T::Output, Error> {
        let ptr = self.state();
        if f(ptr)? != 0 {
            let msg = <String as FromLua>::from_lua(ptr, -1).unwrap_or_default();
            unsafe { sys::lua_pop(ptr, 1) };
            return Err(Error::InvalidSyntax(msg));
        }

        if unsafe { sys::lua_pcall(ptr, 0, <T as ToLua>::len(), 0) } != 0 {
            if let Some(msg) = <String as FromLua>::from_lua(ptr, -1) {
                unsafe { sys::lua_pop(ptr, 1) };
                return Err(Error::LuaError(msg));
            } else {
                unsafe { sys::lua_pop(ptr, 1) };
                return Err(Error::UnknownLuaError);
            }
        } else {
            let size = <T as FromLua>::len();
            let value = T::from_lua(ptr, -size).ok_or(Error::WrongReturnType)?;
            if size > 0 {
                unsafe { sys::lua_pop(ptr, size) };
            }
            Ok(value)
        }
    }

    pub fn set_global(&mut self, name: &str, value: impl ToLua) {
        let ptr = self.state();
        unsafe { sys::lua_pushlstring(ptr, name.as_ptr() as _, name.len()) };
        value.to_lua(ptr);
        unsafe { sys::lua_settable(ptr, sys::LUA_GLOBALSINDEX) };
    }

    pub fn get_global<T: ValueArg>(&self, name: &str) -> Option<T::Output> {
        let ptr = self.state();
        unsafe {
            sys::lua_pushlstring(ptr, name.as_ptr() as _, name.len());
            sys::lua_gettable(ptr, sys::LUA_GLOBALSINDEX);
        }

        let out = T::from_lua(ptr, -1);
        unsafe { sys::lua_pop(ptr, T::len()) };
        out
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

pub trait ValueArg: FromLua {}

macro_rules! impl_get_global {
    ($($ty:ty),*) => { $(impl ValueArg for $ty {} )* };
}

impl_get_global!((), i32, f32, f64, bool, String, StrRef, TableRef);

impl<T> ValueArg for Ud<T> where T: UserData {}

impl<T> ValueArg for Option<T>
where
    T: FromLua,
    T::Output: FromLua,
{
}

impl<I, O> ValueArg for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
}

generate_get_global_tuple_impl!();

pub fn ensure_get_global_impl<T: ValueArg>() {}
