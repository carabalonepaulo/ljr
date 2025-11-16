use luajit2_sys as sys;
use std::marker::PhantomData;

use crate::{error::Error, from_lua::FromLua, to_lua::ToLua};

#[derive(Debug)]
pub struct StackFn<I: FromLua + ToLua, O: FromLua + ToLua> {
    ptr: *mut sys::lua_State,
    idx: i32,
    marker: PhantomData<(I, O)>,
}

impl<I: FromLua + ToLua, O: FromLua + ToLua> StackFn<I, O> {
    pub fn new(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self {
            ptr,
            idx,
            marker: PhantomData,
        }
    }

    pub fn call(&self, args: I) -> Result<O::Output, Error> {
        unsafe { sys::lua_pushvalue(self.ptr, self.idx) };
        args.to_lua(self.ptr);

        if unsafe { sys::lua_pcall(self.ptr, <I as ToLua>::len(), <O as FromLua>::len(), 0) } != 0 {
            if let Some(msg) = <String as FromLua>::from_lua(self.ptr, -1) {
                unsafe { sys::lua_pop(self.ptr, 1) };
                return Err(Error::LuaError(msg));
            } else {
                unsafe { sys::lua_pop(self.ptr, 1) };
                return Err(Error::UnknownLuaError);
            }
        } else {
            if let Some(value) = O::from_lua(self.ptr, <O as FromLua>::len() * -1) {
                Ok(value)
            } else {
                Err(Error::WrongReturnType)
            }
        }
    }
}

impl<I, O> FromLua for StackFn<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    type Output = StackFn<I, O>;

    fn from_lua(ptr: *mut luajit2_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isfunction(ptr, idx) } != 0 {
            Some(StackFn::new(ptr, idx))
        } else {
            None
        }
    }
}
