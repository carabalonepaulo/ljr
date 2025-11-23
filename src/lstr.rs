use std::{marker::PhantomData, rc::Rc};

use crate::{Borrowed, Mode, Owned, from_lua::FromLua, sys, to_lua::ToLua};

pub type StackStr = LStr<Borrowed>;

pub type StrRef = LStr<Owned>;

#[derive(Debug)]
pub struct OwnedLStr<M: Mode>(*mut sys::lua_State, i32, PhantomData<M>);

impl<M> Drop for OwnedLStr<M>
where
    M: Mode,
{
    fn drop(&mut self) {
        unsafe { sys::luaL_unref(self.0, sys::LUA_REGISTRYINDEX, self.1) };
    }
}

#[derive(Debug)]
pub enum LStr<M: Mode> {
    Borrowed(*mut sys::lua_State, i32),
    Owned(Rc<OwnedLStr<M>>),
}

impl<M> LStr<M>
where
    M: Mode,
{
    pub(crate) fn new(ptr: *mut sys::lua_State, value: &str) -> Self {
        value.to_lua(ptr);
        let id = unsafe { sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX) };
        Self::Owned(Rc::new(OwnedLStr(ptr, id, PhantomData)))
    }

    pub fn borrowed(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self::Borrowed(ptr, idx)
    }

    pub fn owned(ptr: *mut sys::lua_State, idx: i32) -> Self {
        unsafe {
            sys::lua_pushvalue(ptr, idx);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            Self::Owned(Rc::new(OwnedLStr(ptr, id, PhantomData)))
        }
    }

    pub fn to_owned(&self) -> Self {
        match self {
            LStr::Borrowed(ptr, idx) => Self::owned(*ptr, *idx),
            LStr::Owned(inner) => Self::Owned(inner.clone()),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        let ptr = match self {
            LStr::Borrowed(ptr, idx) => {
                unsafe { sys::lua_pushvalue(*ptr, *idx) };
                *ptr
            }
            LStr::Owned(inner) => {
                unsafe { sys::lua_rawgeti(inner.0, sys::LUA_REGISTRYINDEX, inner.1 as _) };
                inner.0
            }
        };

        let mut len: usize = 0;
        let str_ptr = unsafe { sys::lua_tolstring(ptr, -1, &mut len) };
        let slice = unsafe { std::slice::from_raw_parts(str_ptr as *const u8, len) };
        unsafe { sys::lua_pop(ptr, 1) };

        slice
    }

    pub fn as_str(&self) -> Option<&str> {
        str::from_utf8(self.as_slice()).ok()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl<M> Clone for LStr<M>
where
    M: Mode,
{
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl FromLua for StackStr {
    type Output = StackStr;

    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            Some(Self::borrowed(ptr, idx))
        } else {
            None
        }
    }
}

impl FromLua for StrRef {
    type Output = StrRef;

    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self::Output> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            Some(Self::owned(ptr, idx))
        } else {
            None
        }
    }
}

impl<M> ToLua for LStr<M>
where
    M: Mode,
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        match self {
            LStr::Borrowed(_, idx) => unsafe {
                sys::lua_pushvalue(ptr, idx);
            },
            LStr::Owned(inner) => unsafe {
                sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _);
            },
        }
    }
}
