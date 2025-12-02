use macros::generate_value_arg_tuple_impl;

use crate::{
    Borrowed,
    func::FnRef,
    lstr::StrRef,
    sys,
    table::TableRef,
    ud::{Ud, UdRef},
};
use std::{cell::Cell, ffi::CString, fmt::Display, ptr, rc::Rc};

use crate::{
    AnyLuaFunction, AnyNativeFunction, AnyUserData, Coroutine, LightUserData, Nil, UserData,
    error::Error, from_lua::FromLua, is_type::IsType, table::Table, to_lua::ToLua,
};

static CTX_KEY: u8 = 0;

unsafe extern "C-unwind" fn individual_sentinel_gc(ptr: *mut sys::lua_State) -> i32 {
    unsafe {
        let weak_ptr = sys::lua_touserdata(ptr, 1) as *mut std::rc::Weak<InnerLua>;
        if !weak_ptr.is_null() {
            let weak = &*weak_ptr;
            if let Some(inner) = weak.upgrade() {
                inner.state.set(ptr::null_mut());
            }
            ptr::drop_in_place(weak_ptr);
        }
        0
    }
}

#[derive(Debug)]
pub struct InnerLua {
    state: Cell<*mut sys::lua_State>,
    owned: bool,
    thread_ref: Option<i32>,
    cache_key: *mut std::ffi::c_void,
    vm_id: *const std::ffi::c_void,
}

pub(crate) unsafe fn get_vm_id(ptr: *mut sys::lua_State) -> *const std::ffi::c_void {
    unsafe {
        sys::lua_pushvalue(ptr, sys::LUA_REGISTRYINDEX);
        let id = sys::lua_topointer(ptr, -1);
        sys::lua_pop(ptr, 1);
        id
    }
}

impl InnerLua {
    unsafe fn create_and_cache_sentinel(
        ptr: *mut sys::lua_State,
        key: *mut std::ffi::c_void,
        inner: Rc<InnerLua>,
    ) {
        unsafe {
            sys::lua_pushlightuserdata(ptr, key);
            let size = std::mem::size_of::<std::rc::Weak<InnerLua>>();
            let udata = sys::lua_newuserdata(ptr, size) as *mut std::rc::Weak<InnerLua>;

            ptr::write(udata, Rc::downgrade(&inner));

            if sys::luaL_newmetatable(ptr, c"__LJR_GUARD".as_ptr()) == 1 {
                sys::lua_pushstring(ptr, c"__gc".as_ptr());
                sys::lua_pushcfunction(ptr, individual_sentinel_gc);
                sys::lua_settable(ptr, -3);
            }
            sys::lua_setmetatable(ptr, -2);

            sys::lua_settable(ptr, sys::LUA_REGISTRYINDEX);
        }
    }

    pub(crate) fn from_ptr(ptr: *mut sys::lua_State) -> Rc<Self> {
        unsafe {
            let is_main = sys::lua_pushthread(ptr) == 1;
            let thread_val_on_stack = !is_main;

            let cache_key = if is_main {
                sys::lua_pop(ptr, 1);
                &CTX_KEY as *const u8 as *mut std::ffi::c_void
            } else {
                ptr as *mut std::ffi::c_void
            };

            sys::lua_pushlightuserdata(ptr, cache_key);
            sys::lua_gettable(ptr, sys::LUA_REGISTRYINDEX);

            let data_ptr = sys::lua_touserdata(ptr, -1);
            sys::lua_pop(ptr, 1);

            if !data_ptr.is_null() {
                let weak_ptr = data_ptr as *mut std::rc::Weak<InnerLua>;
                let weak = &*weak_ptr;

                if let Some(rc) = weak.upgrade() {
                    if thread_val_on_stack {
                        sys::lua_pop(ptr, 1);
                    }
                    return rc;
                }
            }

            let thread_ref = if is_main {
                None
            } else {
                Some(sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX))
            };

            let inner = Rc::new(InnerLua {
                state: Cell::new(ptr),
                owned: false,
                thread_ref,
                cache_key,
                vm_id: get_vm_id(ptr),
            });

            Self::create_and_cache_sentinel(ptr, cache_key, inner.clone());
            inner
        }
    }

    pub(crate) unsafe fn push_ref(&self, dest_ptr: *mut sys::lua_State, id: i32) {
        let _ = self.state();
        let target_vm_id = unsafe { crate::lua::get_vm_id(dest_ptr) };

        if self.vm_id != target_vm_id {
            panic!("unsafe cross-vm operation, value belongs to a different Lua state")
        }

        unsafe { sys::lua_rawgeti(dest_ptr, sys::LUA_REGISTRYINDEX, id as _) };
    }

    pub(crate) fn state(&self) -> *mut sys::lua_State {
        let ptr = self.state.get();
        if ptr.is_null() {
            panic!("lua state has been closed");
        }
        ptr
    }

    pub(crate) fn try_state(&self) -> Option<*mut sys::lua_State> {
        let ptr = self.state.get();
        if ptr.is_null() { None } else { Some(ptr) }
    }
}

impl Drop for InnerLua {
    fn drop(&mut self) {
        let ptr = self.state.get();
        if !ptr.is_null() {
            unsafe {
                if let Some(r) = self.thread_ref {
                    sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, r);
                }

                if self.owned {
                    sys::lua_close(ptr);
                } else {
                    sys::lua_pushlightuserdata(ptr, self.cache_key);
                    sys::lua_pushnil(ptr);
                    sys::lua_settable(ptr, sys::LUA_REGISTRYINDEX);
                }
            }
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
        if ptr.is_null() {
            panic!("lua out of memory: failed to create state");
        }

        let cache_key = &CTX_KEY as *const u8 as *mut std::ffi::c_void;
        let inner = Rc::new(InnerLua {
            state: Cell::new(ptr),
            owned: true,
            thread_ref: None,
            cache_key,
            vm_id: unsafe { get_vm_id(ptr) },
        });

        unsafe { InnerLua::create_and_cache_sentinel(ptr, cache_key, inner.clone()) };

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

    pub fn create_ref<T: UserData>(&self, value: T) -> UdRef<T> {
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

    pub fn do_file<T: ValueArg + FromLua + ToLua>(&mut self, file_name: &str) -> Result<T, Error> {
        self.eval::<T, _>(|ptr| {
            let file_name = CString::new(file_name)?;
            Ok(unsafe { sys::luaL_loadfile(ptr, file_name.as_ptr() as _) })
        })
    }

    pub fn do_string<T: ValueArg + FromLua + ToLua>(&mut self, code: &str) -> Result<T, Error> {
        self.eval::<T, _>(|ptr| {
            let cstring = CString::new(code)?;
            Ok(unsafe { sys::luaL_loadstring(ptr, cstring.as_ptr() as _) })
        })
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

    pub fn get_global<T: FromLua + ValueArg>(&self, name: &str) -> Option<T> {
        let ptr = self.state();
        unsafe {
            sys::lua_pushlstring(ptr, name.as_ptr() as _, name.len());
            sys::lua_gettable(ptr, sys::LUA_GLOBALSINDEX);
        }

        let out = T::from_lua(ptr, -1);
        unsafe { sys::lua_pop(ptr, T::len()) };
        out
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
