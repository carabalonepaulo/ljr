use crate::{lua::FunctionReturnValue, sys};
use std::{marker::PhantomData, rc::Rc};

use crate::{error::Error, from_lua::FromLua, to_lua::ToLua};

#[derive(Debug)]
struct Inner<I: FromLua + ToLua, O: FunctionReturnValue> {
    ptr: *mut sys::lua_State,
    id: i32,
    marker: PhantomData<(I, O)>,
}

#[derive(Debug, Clone)]
pub struct FnRef<I: FromLua + ToLua, O: FunctionReturnValue>(Rc<Inner<I, O>>);

impl<I: FromLua + ToLua, O: FunctionReturnValue> FnRef<I, O> {
    pub fn from_stack(ptr: *mut sys::lua_State, idx: i32) -> Self {
        unsafe { sys::lua_pushvalue(ptr, idx) };
        let id = unsafe { crate::sys::luaL_ref(ptr, crate::sys::LUA_REGISTRYINDEX) };
        Self(Rc::new(Inner {
            ptr,
            id,
            marker: PhantomData,
        }))
    }

    pub fn call(&self, args: I) -> Result<O::Output, Error> {
        let ptr = self.0.ptr;
        let id = self.0.id;

        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, id as _) };
        args.to_lua(ptr);

        if unsafe {
            sys::lua_pcall(
                ptr,
                <I as ToLua>::len(),
                <O as FunctionReturnValue>::len(),
                0,
            )
        } != 0
        {
            if let Some(msg) = <String as FromLua>::from_lua(ptr, -1) {
                unsafe { sys::lua_pop(ptr, 1) };
                return Err(Error::LuaError(msg));
            } else {
                unsafe { sys::lua_pop(ptr, 1) };
                return Err(Error::UnknownLuaError);
            }
        } else {
            O::from_lua(ptr, -<O as FunctionReturnValue>::len()).ok_or(Error::WrongReturnType)
        }
    }
}

impl<I, O> FromLua for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FunctionReturnValue,
{
    type Output = FnRef<I, O>;

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_isfunction(ptr, idx) } != 0 {
            Some(FnRef::from_stack(ptr, idx))
        } else {
            None
        }
    }
}

impl<I, O> ToLua for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FunctionReturnValue,
{
    fn to_lua(self, ptr: *mut sys::lua_State) {
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.0.id as _) };
    }

    fn len() -> i32 {
        1
    }
}
