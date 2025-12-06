use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
};

use crate::{
    Borrowed, Mode, Owned,
    from_lua::FromLua,
    is_type::IsType,
    lua::InnerLua,
    owned_value::{LuaInnerHandle, OwnedValue},
    sys,
    to_lua::ToLua,
};

pub trait StringState {
    type State;
}

pub trait StringAccess {
    fn as_slice<'a>(&'a self) -> &'a [u8];

    fn as_str<'a>(&'a self) -> Option<&'a str> {
        str::from_utf8(self.as_slice()).ok()
    }
}

pub struct BorrowedState {
    ptr: *mut sys::lua_State,
    idx: i32,
    slice: &'static [u8],
}

impl StringState for Borrowed {
    type State = BorrowedState;
}

impl StringAccess for BorrowedState {
    fn as_slice<'a>(&'a self) -> &'a [u8] {
        self.slice
    }
}

#[derive(Debug)]
pub struct OwnedState {
    lua: RefCell<Rc<InnerLua>>,
    id: i32,
    slice: &'static [u8],
}

impl Drop for OwnedState {
    fn drop(&mut self) {
        if let Some(ptr) = self.lua.borrow().try_state() {
            unsafe { sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, self.id) };
        }
    }
}

impl StringState for Owned {
    type State = OwnedState;
}

impl StringAccess for OwnedState {
    fn as_slice<'a>(&'a self) -> &'a [u8] {
        let _ = self.lua.borrow().state();
        self.slice
    }
}

pub type StackStr = LStr<Borrowed>;
pub type StrRef = LStr<Owned>;

pub struct LStr<M>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    state: M::State,
}

impl<M> LStr<M>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    pub fn as_slice<'a>(&'a self) -> &'a [u8] {
        self.state.as_slice()
    }

    pub fn as_str<'a>(&'a self) -> Option<&'a str> {
        self.state.as_str()
    }
}

impl StackStr {
    pub fn from_stack(ptr: *mut sys::lua_State, idx: i32) -> StackStr {
        let idx = unsafe { sys::lua_absindex(ptr, idx) };
        let slice = slice_from_stack(ptr, idx);
        Self {
            state: BorrowedState { ptr, idx, slice },
        }
    }

    pub fn to_owned(&self) -> StrRef {
        StrRef::from_stack(self.state.ptr, self.state.idx)
    }
}

impl StrRef {
    pub fn new(lua: Rc<InnerLua>, value: &str) -> StrRef {
        let ptr = lua.state();
        value.to_lua(ptr);
        let slice = slice_from_stack(ptr, -1);
        let id = unsafe { sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX) };
        let lua = RefCell::new(lua);
        Self {
            state: OwnedState { lua, id, slice },
        }
    }

    pub fn from_stack(ptr: *mut sys::lua_State, idx: i32) -> StrRef {
        unsafe {
            let lua = RefCell::new(InnerLua::from_ptr(ptr));
            sys::lua_pushvalue(ptr, idx);
            let slice = slice_from_stack(ptr, -1);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);

            Self {
                state: OwnedState { lua, id, slice },
            }
        }
    }
}

impl Clone for StrRef {
    fn clone(&self) -> Self {
        let lua = self.state.lua.clone();

        let slice = self.state.slice;
        let ptr = lua.borrow().state();
        let id = unsafe {
            sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _);
            sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX)
        };

        let state = OwnedState { id, lua, slice };
        Self { state }
    }
}

impl<M> std::fmt::Display for LStr<M>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8_lossy(self.as_slice());
        write!(f, "{}", s)
    }
}

impl<M> std::fmt::Debug for LStr<M>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slice = self.as_slice();
        f.debug_struct("LStr")
            .field("ptr", &slice.as_ptr())
            .field("len", &slice.len())
            .field("preview", &String::from_utf8_lossy(slice))
            .finish()
    }
}

impl<M> AsRef<[u8]> for LStr<M>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<M> From<&LStr<M>> for Vec<u8>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    fn from(value: &LStr<M>) -> Self {
        value.state.as_slice().to_vec()
    }
}

impl<M> From<LStr<M>> for Vec<u8>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    fn from(value: LStr<M>) -> Self {
        value.state.as_slice().to_vec()
    }
}

unsafe impl FromLua for StackStr {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<StackStr> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            Some(StackStr::from_stack(ptr, idx))
        } else {
            None
        }
    }
}

unsafe impl FromLua for StrRef {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32 {
            Some(StrRef::from_stack(ptr, idx))
        } else {
            None
        }
    }
}

unsafe impl ToLua for &StackStr {
    unsafe fn to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) {
        InnerLua::ensure_context_raw(self.state.ptr, ptr);
        unsafe { sys::lua_pushvalue(ptr, self.state.idx) };
    }
}

unsafe impl ToLua for &StrRef {
    unsafe fn to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) {
        InnerLua::ensure_context_raw(self.state.lua.borrow().state(), ptr);
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _) };
    }
}

unsafe impl ToLua for StackStr {
    unsafe fn to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) {
        unsafe { (&self).to_lua_unchecked(ptr) };
    }
}

unsafe impl ToLua for StrRef {
    unsafe fn to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) {
        unsafe { (&self).to_lua_unchecked(ptr) };
    }
}

impl<M> IsType for LStr<M>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        (unsafe { sys::lua_type(ptr, idx) } == sys::LUA_TSTRING as i32)
    }
}

impl<M1: Mode, M2: Mode> PartialEq<LStr<M2>> for LStr<M1>
where
    M1: Mode + StringState,
    M1::State: StringAccess,
    M2: Mode + StringState,
    M2::State: StringAccess,
{
    fn eq(&self, other: &LStr<M2>) -> bool {
        self.state.as_slice().as_ptr() == other.state.as_slice().as_ptr()
    }
}

impl<M: Mode> Eq for LStr<M>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
}

impl<M: Mode> Hash for LStr<M>
where
    M: Mode + StringState,
    M::State: StringAccess,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.as_slice().as_ptr().hash(state);
    }
}

fn slice_from_stack(ptr: *mut sys::lua_State, idx: i32) -> &'static [u8] {
    let mut len: usize = 0;
    let str_ptr = unsafe { sys::lua_tolstring(ptr, idx, &mut len) };
    let slice = unsafe { std::slice::from_raw_parts(str_ptr as *const u8, len) };
    slice
}

impl crate::owned_value::private::Sealed for StrRef {}

impl OwnedValue for StrRef {
    fn handle(&self) -> LuaInnerHandle<'_> {
        LuaInnerHandle(&self.state.lua)
    }
}
