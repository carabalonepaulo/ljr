use std::{marker::PhantomData, rc::Rc};

use crate::{Borrowed, Mode, Owned, from_lua::FromLua, lua::InnerLua, sys, to_lua::ToLua};

pub type StackStr = LStr<Borrowed>;

pub type StrRef = LStr<Owned>;

#[derive(Debug)]
pub struct OwnedLStr<M: Mode>(Rc<InnerLua>, i32, PhantomData<M>);

impl<M> Drop for OwnedLStr<M>
where
    M: Mode,
{
    fn drop(&mut self) {
        let ptr = self.0.state_or_null();
        if !ptr.is_null() {
            unsafe { sys::luaL_unref(self.0.state(), sys::LUA_REGISTRYINDEX, self.1) };
        }
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
    pub(crate) fn new(inner: Rc<InnerLua>, value: &str) -> Self {
        let ptr = inner.state();
        value.to_lua(ptr);
        let id = unsafe { sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX) };
        Self::Owned(Rc::new(OwnedLStr(inner, id, PhantomData)))
    }

    pub(crate) fn borrowed(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self::Borrowed(ptr, unsafe { sys::lua_absindex(ptr, idx) })
    }

    pub(crate) fn owned(inner: Rc<InnerLua>, idx: i32) -> Self {
        unsafe {
            let ptr = inner.state();
            sys::lua_pushvalue(ptr, idx);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            Self::Owned(Rc::new(OwnedLStr(inner, id, PhantomData)))
        }
    }

    pub fn to_owned(&self) -> Self {
        match self {
            LStr::Borrowed(ptr, idx) => Self::owned(InnerLua::from_ptr(*ptr), *idx),
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
                let ptr = inner.0.state();
                unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _) };
                ptr
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
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            Some(Self::borrowed(ptr, idx))
        } else {
            None
        }
    }
}

impl FromLua for StrRef {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            Some(Self::owned(InnerLua::from_ptr(ptr), idx))
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
