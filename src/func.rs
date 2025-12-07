use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    Borrowed, Mode, Owned,
    error::{Error, UnwrapDisplay},
    from_lua::FromLua,
    helper,
    lua::{InnerLua, ValueArg},
    owned_value::LuaInnerHandle,
    prelude::OwnedValue,
    stack_guard::StackGuard,
    sys,
    to_lua::ToLua,
};

pub trait FuncState<I, O> {
    type State;
}

pub trait FuncAccess {
    fn try_ptr(&self) -> Result<*mut sys::lua_State, Error>;

    fn fn_ptr(&self) -> *const std::ffi::c_void;

    fn push_fn(&self, ptr: *mut sys::lua_State);

    fn try_call<I: ToLua, O: FromLua + ValueArg>(&self, args: I) -> Result<O, Error> {
        unsafe {
            let ptr = self.try_ptr()?;
            helper::try_check_stack(ptr, I::LEN + O::LEN)?;
            let _g = StackGuard::new(ptr);

            self.push_fn(ptr);
            args.to_lua_unchecked(ptr);

            if sys::lua_pcall(ptr, I::LEN, O::LEN, 0) != 0 {
                Err(Error::from_stack(ptr, -1))
            } else {
                O::try_from_lua(ptr, O::LEN * -1)
            }
        }
    }

    fn try_call_then<I: ToLua, O: FromLua, F: FnOnce(&O) -> R, R>(
        &self,
        args: I,
        f: F,
    ) -> Result<R, Error> {
        unsafe {
            let ptr = self.try_ptr()?;
            helper::try_check_stack(ptr, I::LEN + O::LEN)?;
            let _g = StackGuard::new(ptr);

            self.push_fn(ptr);
            args.to_lua_unchecked(ptr);

            if sys::lua_pcall(ptr, I::LEN, O::LEN, 0) != 0 {
                Err(Error::from_stack(ptr, -1))
            } else {
                O::try_from_lua(ptr, O::LEN * -1).map(|v| f(&v))
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
    fn try_ptr(&self) -> Result<*mut sys::lua_State, Error> {
        Ok(self.ptr)
    }

    fn fn_ptr(&self) -> *const std::ffi::c_void {
        self.fn_ptr
    }

    fn push_fn(&self, ptr: *mut sys::lua_State) {
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
        if let Ok(ptr) = self.lua.borrow().try_state() {
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
    fn try_ptr(&self) -> Result<*mut sys::lua_State, Error> {
        self.lua.borrow().try_state()
    }

    fn fn_ptr(&self) -> *const std::ffi::c_void {
        self.fn_ptr
    }

    fn push_fn(&self, ptr: *mut sys::lua_State) {
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
        self.state.try_call(args)
    }

    pub fn call_then<F: FnOnce(&O) -> R, R>(&self, args: I, f: F) -> Result<R, Error> {
        self.state.try_call_then(args, f)
    }
}

impl<I, O> FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn try_clone(&self) -> Result<Self, Error> {
        let lua = self.state.lua.clone();
        let fn_ptr = self.state.fn_ptr;
        let id = unsafe {
            let ptr = lua.borrow().try_state()?;
            sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _);
            sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX)
        };
        Ok(Self {
            state: OwnedState {
                lua,
                id,
                fn_ptr,
                marker: PhantomData,
            },
        })
    }
}

impl<I, O> Clone for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn clone(&self) -> Self {
        self.try_clone().unwrap_display()
    }
}

unsafe impl<I, O> FromLua for StackFn<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn try_from_lua(ptr: *mut sys::lua_State, idx: i32) -> Result<Self, Error> {
        unsafe {
            let idx = sys::lua_absindex(ptr, idx);
            if sys::lua_isfunction(ptr, idx) != 0 {
                let fn_ptr = sys::lua_topointer(ptr, idx);
                Ok(StackFn {
                    state: BorrowedState {
                        ptr,
                        idx,
                        fn_ptr,
                        marker: PhantomData,
                    },
                })
            } else {
                Err(Error::UnexpectedType)
            }
        }
    }
}

unsafe impl<I, O> FromLua for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn try_from_lua(ptr: *mut sys::lua_State, idx: i32) -> Result<Self, Error> {
        unsafe {
            let idx = sys::lua_absindex(ptr, idx);
            if sys::lua_isfunction(ptr, idx) != 0 {
                let lua = RefCell::new(InnerLua::from_ptr(ptr));
                let fn_ptr = sys::lua_topointer(ptr, idx);
                sys::lua_pushvalue(ptr, idx);
                let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
                Ok(FnRef {
                    state: OwnedState {
                        lua,
                        id,
                        fn_ptr,
                        marker: PhantomData,
                    },
                })
            } else {
                Err(Error::UnexpectedType)
            }
        }
    }
}

unsafe impl<I, O> ToLua for &StackFn<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    unsafe fn try_to_lua_unchecked(self, ptr: *mut sys::lua_State) -> Result<(), Error> {
        InnerLua::try_ensure_context_raw(self.state.ptr, ptr)?;
        unsafe { sys::lua_pushvalue(ptr, self.state.idx) };
        Ok(())
    }
}

unsafe impl<I, O> ToLua for StackFn<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    unsafe fn try_to_lua_unchecked(self, ptr: *mut sys::lua_State) -> Result<(), Error> {
        unsafe { (&self).try_to_lua_unchecked(ptr) }
    }
}

unsafe impl<I, O> ToLua for &FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    unsafe fn try_to_lua_unchecked(self, ptr: *mut sys::lua_State) -> Result<(), Error> {
        InnerLua::try_ensure_context_raw(self.state.lua.borrow().try_state()?, ptr)?;
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _) };
        Ok(())
    }
}

unsafe impl<I, O> ToLua for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    unsafe fn try_to_lua_unchecked(self, ptr: *mut sys::lua_State) -> Result<(), Error> {
        unsafe { (&self).try_to_lua_unchecked(ptr) }
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
