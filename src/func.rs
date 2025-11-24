use std::{marker::PhantomData, rc::Rc};

use crate::{
    Borrowed, Mode, Owned, error::Error, from_lua::FromLua, lua::InnerLua, sys, to_lua::ToLua,
};

pub type StackFn<I, O> = Func<Borrowed, I, O>;

pub type FnRef<I, O> = Func<Owned, I, O>;

pub struct OwnedFunc<M: Mode, I: FromLua + ToLua, O: FromLua + ToLua>(
    Rc<InnerLua>,
    i32,
    PhantomData<(M, I, O)>,
);

impl<M, I, O> Drop for OwnedFunc<M, I, O>
where
    M: Mode,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn drop(&mut self) {
        let ptr = self.0.state_or_null();
        if !ptr.is_null() {
            unsafe { sys::luaL_unref(self.0.state(), sys::LUA_REGISTRYINDEX, self.1) }
        }
    }
}

pub enum Func<M: Mode, I: FromLua + ToLua, O: FromLua + ToLua> {
    Borrowed(*mut sys::lua_State, i32),
    Owned(Rc<OwnedFunc<M, I, O>>),
}

impl<M, I, O> Func<M, I, O>
where
    M: Mode,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    pub(crate) fn borrowed(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self::Borrowed(ptr, unsafe { sys::lua_absindex(ptr, idx) })
    }

    pub(crate) fn owned(inner: Rc<InnerLua>, idx: i32) -> Self {
        unsafe {
            let ptr = inner.state();
            let idx = sys::lua_absindex(ptr, idx);
            sys::lua_pushvalue(ptr, idx);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            Self::Owned(Rc::new(OwnedFunc(inner, id, PhantomData)))
        }
    }

    pub fn to_owned(&self) -> Self {
        match self {
            Func::Borrowed(ptr, idx) => Self::owned(InnerLua::from_ptr(*ptr), *idx),
            Func::Owned(ud) => Self::Owned(ud.clone()),
        }
    }

    pub fn call(&self, args: I) -> Result<O, Error> {
        unsafe {
            let ptr = match self {
                Func::Borrowed(ptr, idx) => {
                    sys::lua_pushvalue(*ptr, *idx);
                    *ptr
                }
                Func::Owned(inner) => {
                    let ptr = inner.0.state();
                    sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _);
                    ptr
                }
            };
            let old_top = sys::lua_gettop(ptr) - 1;

            args.to_lua(ptr);
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
                if let Some(value) = O::from_lua(ptr, o_len * -1) {
                    Ok(value)
                } else {
                    let diff = sys::lua_gettop(ptr) - old_top;
                    sys::lua_pop(ptr, diff);
                    Err(Error::WrongReturnType)
                }
            }
        }
    }
}

impl<M, I, O> Clone for Func<M, I, O>
where
    M: Mode,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl<I, O> FromLua for FnRef<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_isfunction(ptr, idx) } != 0 {
            Some(Func::owned(InnerLua::from_ptr(ptr), idx))
        } else {
            None
        }
    }
}

impl<I, O> FromLua for StackFn<I, O>
where
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_isfunction(ptr, idx) } != 0 {
            Some(Func::borrowed(ptr, idx))
        } else {
            None
        }
    }
}

impl<M, I, O> ToLua for Func<M, I, O>
where
    M: Mode,
    I: FromLua + ToLua,
    O: FromLua + ToLua,
{
    fn to_lua(self, ptr: *mut sys::lua_State) {
        unsafe {
            match self {
                Func::Borrowed(_, idx) => sys::lua_pushvalue(ptr, idx),
                Func::Owned(inner) => {
                    sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _);
                }
            }
        }
    }
}
