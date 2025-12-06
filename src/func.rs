use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    Borrowed, Mode, Owned,
    error::Error,
    from_lua::FromLua,
    lua::{InnerLua, ValueArg},
    owned_value::LuaInnerHandle,
    prelude::OwnedValue,
    sys,
    to_lua::ToLua,
};

pub trait FuncState<I, O> {
    type State;
}

pub trait FuncAccess {
    fn ptr(&self) -> *mut sys::lua_State;

    fn fn_ptr(&self) -> *const std::ffi::c_void;

    fn push_fn(&self, ptr: *mut sys::lua_State);

    fn call<I: FromLua + ToLua, O: FromLua + ToLua + ValueArg>(&self, args: I) -> Result<O, Error> {
        unsafe {
            let ptr = self.ptr();
            let old_top = sys::lua_gettop(ptr);
            self.push_fn(ptr);

            args.to_lua_unchecked(ptr);
            let o_len = <O as FromLua>::len();

            if sys::lua_pcall(ptr, <I as ToLua>::len(), o_len, 0) != 0 {
                if let Some(msg) = <String as FromLua>::from_lua(ptr, -1) {
                    sys::lua_pop(ptr, 1);
                    return Err(Error::LuaError(msg));
                } else {
                    sys::lua_pop(ptr, 1);
                    return Err(Error::UnknownLuaError);
                }
            } else {
                let result = if let Some(value) = O::from_lua(ptr, o_len * -1) {
                    Ok(value)
                } else {
                    Err(Error::WrongReturnType)
                };

                sys::lua_settop(ptr, old_top);
                result
            }
        }
    }

    fn call_then<I: FromLua + ToLua, O: FromLua + ToLua, F: FnOnce(&O) -> R, R>(
        &self,
        args: I,
        f: F,
    ) -> Result<R, Error> {
        unsafe {
            let ptr = self.ptr();
            let old_top = sys::lua_gettop(ptr);
            self.push_fn(ptr);

            args.to_lua_unchecked(ptr);
            let o_len = <O as FromLua>::len();

            if sys::lua_pcall(ptr, <I as ToLua>::len(), o_len, 0) != 0 {
                if let Some(msg) = <String as FromLua>::from_lua(ptr, -1) {
                    sys::lua_pop(ptr, 1);
                    return Err(Error::LuaError(msg));
                } else {
                    sys::lua_pop(ptr, 1);
                    return Err(Error::UnknownLuaError);
                }
            } else {
                let value = O::from_lua(ptr, o_len * -1);
                let result = if let Some(val) = value {
                    Ok(f(&val))
                } else {
                    Err(Error::WrongReturnType)
                };

                sys::lua_settop(ptr, old_top);
                result
            }
        }
    }
}

pub struct BorrowedState<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    ptr: *mut sys::lua_State,
    idx: i32,
    fn_ptr: *const std::ffi::c_void,
    marker: PhantomData<(I, O)>,
}

impl<I, O> FuncState<I, O> for Borrowed
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    type State = BorrowedState<I, O>;
}

impl<I, O> FuncAccess for BorrowedState<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn ptr(&self) -> *mut mlua_sys::lua_State {
        self.ptr
    }

    fn fn_ptr(&self) -> *const std::ffi::c_void {
        self.fn_ptr
    }

    fn push_fn(&self, ptr: *mut mlua_sys::lua_State) {
        unsafe { sys::lua_pushvalue(ptr, self.idx) };
    }
}

#[derive(Debug)]
pub struct OwnedState<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    lua: RefCell<Rc<InnerLua>>,
    id: i32,
    fn_ptr: *const std::ffi::c_void,
    marker: PhantomData<(I, O)>,
}

impl<I, O> Drop for OwnedState<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn drop(&mut self) {
        if let Some(ptr) = self.lua.borrow().try_state() {
            unsafe { sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, self.id) };
        }
    }
}

impl<I, O> FuncState<I, O> for Owned
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    type State = OwnedState<I, O>;
}

impl<I, O> FuncAccess for OwnedState<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn ptr(&self) -> *mut mlua_sys::lua_State {
        self.lua.borrow().state()
    }

    fn fn_ptr(&self) -> *const std::ffi::c_void {
        self.fn_ptr
    }

    fn push_fn(&self, ptr: *mut mlua_sys::lua_State) {
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.id as _) };
    }
}

pub type StackFn<I, O> = Func<Borrowed, I, O>;
pub type FnRef<I, O> = Func<Owned, I, O>;

pub struct Func<M, I, O>
where
    M: Mode + FuncState<I, O>,
    M::State: FuncAccess,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    state: M::State,
}

impl<M, I, O> Func<M, I, O>
where
    M: Mode + FuncState<I, O>,
    M::State: FuncAccess,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    pub fn call(&self, args: I) -> Result<O, Error>
    where
        O: ValueArg,
    {
        self.state.call(args)
    }

    pub fn call_then<F: FnOnce(&O) -> R, R>(&self, args: I, f: F) -> Result<R, Error> {
        self.state.call_then(args, f)
    }
}

impl<I, O> Clone for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn clone(&self) -> Self {
        let lua = self.state.lua.clone();
        let fn_ptr = self.state.fn_ptr;
        let id = unsafe {
            let ptr = lua.borrow().state();
            sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _);
            sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX)
        };
        Self {
            state: OwnedState {
                lua,
                id,
                fn_ptr,
                marker: PhantomData,
            },
        }
    }
}

unsafe impl<I, O> FromLua for StackFn<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self> {
        unsafe {
            let idx = sys::lua_absindex(ptr, idx);
            if sys::lua_isfunction(ptr, idx) != 0 {
                let fn_ptr = sys::lua_topointer(ptr, idx);
                Some(StackFn {
                    state: BorrowedState {
                        ptr,
                        idx,
                        fn_ptr,
                        marker: PhantomData,
                    },
                })
            } else {
                None
            }
        }
    }
}

unsafe impl<I, O> FromLua for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        unsafe {
            let idx = sys::lua_absindex(ptr, idx);
            if sys::lua_isfunction(ptr, idx) != 0 {
                let lua = RefCell::new(InnerLua::from_ptr(ptr));
                let fn_ptr = sys::lua_topointer(ptr, idx);
                sys::lua_pushvalue(ptr, idx);
                let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
                Some(FnRef {
                    state: OwnedState {
                        lua,
                        id,
                        fn_ptr,
                        marker: PhantomData,
                    },
                })
            } else {
                None
            }
        }
    }
}

unsafe impl<I, O> ToLua for &StackFn<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    unsafe fn to_lua_unchecked(self, ptr: *mut sys::lua_State) {
        InnerLua::ensure_context_raw(self.state.ptr, ptr);
        unsafe { sys::lua_pushvalue(ptr, self.state.idx) };
    }
}

unsafe impl<I, O> ToLua for StackFn<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    unsafe fn to_lua_unchecked(self, ptr: *mut sys::lua_State) {
        unsafe { (&self).to_lua_unchecked(ptr) };
    }
}

unsafe impl<I, O> ToLua for &FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    unsafe fn to_lua_unchecked(self, ptr: *mut sys::lua_State) {
        InnerLua::ensure_context_raw(self.state.lua.borrow().state(), ptr);
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _) };
    }
}

unsafe impl<I, O> ToLua for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    unsafe fn to_lua_unchecked(self, ptr: *mut sys::lua_State) {
        unsafe { (&self).to_lua_unchecked(ptr) };
    }
}

impl<M1, M2, I, O> PartialEq<Func<M2, I, O>> for Func<M1, I, O>
where
    M1: Mode + FuncState<I, O>,
    M1::State: FuncAccess,
    M2: Mode + FuncState<I, O>,
    M2::State: FuncAccess,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn eq(&self, other: &Func<M2, I, O>) -> bool {
        self.state.fn_ptr() == other.state.fn_ptr()
    }
}

impl<M, I, O> Eq for Func<M, I, O>
where
    M: Mode + FuncState<I, O>,
    M::State: FuncAccess,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
}

impl<M, I, O> Hash for Func<M, I, O>
where
    M: Mode + FuncState<I, O>,
    M::State: FuncAccess,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.fn_ptr().hash(state);
    }
}

impl<I, O> crate::owned_value::private::Sealed for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
}

impl<I, O> OwnedValue for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn handle(&self) -> LuaInnerHandle<'_> {
        LuaInnerHandle(&self.state.lua)
    }
}
