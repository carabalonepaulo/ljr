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
        if let Some(ptr) = self.0.try_state() {
            unsafe { sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, self.1) };
        }
    }
}

#[derive(Debug)]
pub enum LStr<M: Mode> {
    Borrowed(*mut sys::lua_State, i32, &'static [u8]),
    Owned(Rc<OwnedLStr<M>>, &'static [u8]),
}

impl<M> LStr<M>
where
    M: Mode,
{
    fn slice_from_stack(ptr: *mut sys::lua_State, idx: i32) -> &'static [u8] {
        let mut len: usize = 0;
        let str_ptr = unsafe { sys::lua_tolstring(ptr, idx, &mut len) };
        let slice = unsafe { std::slice::from_raw_parts(str_ptr as *const u8, len) };
        slice
    }

    pub(crate) fn new(inner: Rc<InnerLua>, value: &str) -> Self {
        let ptr = inner.state();
        value.to_lua(ptr);
        let slice = Self::slice_from_stack(ptr, -1);
        let id = unsafe { sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX) };
        let inner = Rc::new(OwnedLStr(inner, id, PhantomData));
        Self::Owned(inner, slice)
    }

    pub(crate) fn borrowed(ptr: *mut sys::lua_State, idx: i32) -> Self {
        let idx = unsafe { sys::lua_absindex(ptr, idx) };
        let slice = Self::slice_from_stack(ptr, idx);
        Self::Borrowed(ptr, idx, slice)
    }

    pub(crate) fn owned(inner: Rc<InnerLua>, idx: i32, slice: &'static [u8]) -> Self {
        unsafe {
            let ptr = inner.state();
            sys::lua_pushvalue(ptr, idx);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            let owned_str = Rc::new(OwnedLStr(inner, id, PhantomData));
            Self::Owned(owned_str, slice)
        }
    }

    pub fn to_owned(&self) -> Self {
        match self {
            LStr::Borrowed(ptr, idx, slice) => Self::owned(InnerLua::from_ptr(*ptr), *idx, *slice),
            LStr::Owned(inner, slice) => Self::Owned(inner.clone(), slice),
        }
    }

    pub fn as_slice<'a>(&'a self) -> &'a [u8] {
        match self {
            LStr::Borrowed(_, _, slice) => slice,
            LStr::Owned(inner, slice) => {
                let _ = inner.0.state();
                slice
            }
        }
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

unsafe impl FromLua for StackStr {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            Some(Self::borrowed(ptr, idx))
        } else {
            None
        }
    }
}

unsafe impl FromLua for StrRef {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            let slice = Self::slice_from_stack(ptr, idx);
            Some(Self::owned(InnerLua::from_ptr(ptr), idx, slice))
        } else {
            None
        }
    }
}

unsafe impl<M> ToLua for &LStr<M>
where
    M: Mode,
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        match self {
            LStr::Borrowed(_, idx, _) => unsafe {
                sys::lua_pushvalue(ptr, *idx);
            },
            LStr::Owned(inner, _) => unsafe {
                sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _);
            },
        }
    }
}

unsafe impl<M> ToLua for LStr<M>
where
    M: Mode,
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        match self {
            LStr::Borrowed(_, idx, _) => unsafe {
                sys::lua_pushvalue(ptr, idx);
            },
            LStr::Owned(inner, _) => inner.0.push_ref(ptr, inner.1),
        }
    }
}
