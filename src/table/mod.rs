use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{
    Borrowed, Mode, Owned,
    from_lua::FromLua,
    is_type::IsType,
    lua::{InnerLua, ValueArg},
    prelude::TableView,
    sys,
    table::constraints::TableKey,
    to_lua::ToLua,
};

pub mod builder;
pub mod constraints;
pub mod view;

pub trait TableStorage {
    type State;
}

#[doc(hidden)]
pub trait TableAccess {
    unsafe fn get_table_ptr(&self) -> *const std::ffi::c_void;

    fn as_ref<'t>(&'t self) -> Guard<'t>;
    fn as_mut<'t>(&'t mut self) -> GuardMut<'t>;

    fn with<F: FnOnce(&TableView) -> R, R>(&self, f: F) -> R {
        let guard = self.as_ref();
        f(&*guard)
    }

    fn with_mut<F: FnOnce(&mut TableView) -> R, R>(&mut self, f: F) -> R {
        let mut guard = self.as_mut();
        f(&mut *guard)
    }
}

pub struct BorrowedState {
    ptr: *mut sys::lua_State,
    idx: i32,
    table_ptr: *const std::ffi::c_void,
}

impl TableStorage for Borrowed {
    type State = BorrowedState;
}

impl TableAccess for BorrowedState {
    #[inline]
    unsafe fn get_table_ptr(&self) -> *const std::ffi::c_void {
        self.table_ptr
    }

    fn as_ref<'t>(&'t self) -> Guard<'t> {
        let view = TableView::new(self.ptr, self.idx);
        let top = unsafe { sys::lua_gettop(self.ptr) };
        Guard(self.ptr, top, view)
    }

    fn as_mut<'t>(&'t mut self) -> GuardMut<'t> {
        let view = TableView::new(self.ptr, self.idx);
        let top = unsafe { sys::lua_gettop(self.ptr) };
        GuardMut(self.ptr, top, view)
    }
}

#[derive(Debug, Clone)]
pub struct OwnedInner {
    lua: Rc<InnerLua>,
    id: i32,
}

impl Drop for OwnedInner {
    fn drop(&mut self) {
        if let Some(ptr) = self.lua.try_state() {
            unsafe { sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, self.id) };
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedState {
    inner: Rc<OwnedInner>,
    table_ptr: *const std::ffi::c_void,
}

impl TableStorage for Owned {
    type State = OwnedState;
}

impl TableAccess for OwnedState {
    #[inline]
    unsafe fn get_table_ptr(&self) -> *const std::ffi::c_void {
        self.table_ptr
    }

    fn as_ref<'t>(&'t self) -> Guard<'t> {
        unsafe {
            let ptr = self.inner.lua.state();
            let top = sys::lua_gettop(ptr);
            sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.inner.id as _);
            let table_idx = sys::lua_absindex(ptr, -1);
            let view = TableView::new(ptr, table_idx);
            Guard(ptr, top, view)
        }
    }

    fn as_mut<'t>(&'t mut self) -> GuardMut<'t> {
        unsafe {
            let ptr = self.inner.lua.state();
            let top = sys::lua_gettop(ptr);
            sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.inner.id as _);
            let table_idx = sys::lua_absindex(ptr, -1);
            let view = TableView::new(ptr, table_idx);
            GuardMut(ptr, top, view)
        }
    }
}

pub type StackTable = Table<Borrowed>;
pub type TableRef = Table<Owned>;

#[derive(Debug)]
#[repr(transparent)]
pub struct Table<M>
where
    M: Mode + TableStorage,
    M::State: TableAccess,
{
    state: M::State,
}

impl<M> Table<M>
where
    M: Mode + TableStorage,
    M::State: TableAccess,
{
    #[inline]
    pub fn as_ref<'t>(&'t self) -> Guard<'t> {
        self.state.as_ref()
    }

    #[inline]
    pub fn as_mut<'t>(&'t mut self) -> GuardMut<'t> {
        self.state.as_mut()
    }

    #[inline]
    pub fn with<F: FnOnce(&TableView) -> R, R>(&self, f: F) -> R {
        self.state.with(f)
    }

    #[inline]
    pub fn with_mut<F: FnOnce(&mut TableView) -> R, R>(&mut self, f: F) -> R {
        self.state.with_mut(f)
    }

    #[inline]
    pub fn push(&mut self, value: impl ToLua) {
        self.with_mut(|t| t.push(value));
    }

    #[inline]
    pub fn pop<T: FromLua + ValueArg>(&mut self) -> Option<T> {
        self.with_mut(|t| t.pop())
    }

    #[inline]
    pub fn insert(&mut self, index: i32, value: impl ToLua) {
        self.with_mut(|t| t.insert(index, value));
    }

    #[inline]
    pub fn remove<T: FromLua + ValueArg>(&mut self, index: i32) -> Option<T> {
        self.with_mut(|t| t.remove(index))
    }

    #[inline]
    pub fn clear(&mut self) {
        self.with_mut(|t| t.clear());
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.with(|t| t.len())
    }

    pub fn for_each<K, V, F>(&self, f: F)
    where
        K: FromLua,
        V: FromLua,
        F: FnMut(&K, &V) -> bool,
    {
        self.with(|t| t.for_each(f))
    }

    pub fn extend_from_slice<T: ToLua + Clone>(&mut self, src: &[T]) {
        let mut guard = self.as_mut();
        src.iter().for_each(|v| guard.push(v.clone()));
    }

    pub fn extend_from_map<'a, K: TableKey<'a> + Clone, V: FromLua + ToLua + Clone>(
        &mut self,
        src: &HashMap<K, V>,
    ) {
        let mut guard = self.as_mut();
        src.iter()
            .for_each(|(k, v)| guard.set(k.clone(), v.clone()));
    }
}

impl StackTable {
    pub fn from_stack(ptr: *mut sys::lua_State, idx: i32) -> StackTable {
        unsafe {
            let idx = sys::lua_absindex(ptr, idx);
            let table_ptr = sys::lua_topointer(ptr, idx);
            Self {
                state: BorrowedState {
                    ptr,
                    idx,
                    table_ptr,
                },
            }
        }
    }

    pub fn to_owned(&self) -> TableRef {
        Table::<Owned>::from_stack(self.state.ptr, self.state.idx)
    }
}

impl TableRef {
    pub fn new(lua: Rc<InnerLua>) -> TableRef {
        unsafe {
            let ptr = lua.state();
            sys::lua_newtable(ptr);
            let table_ptr = sys::lua_topointer(ptr, -1);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            let inner = Rc::new(OwnedInner { lua, id });
            Self {
                state: OwnedState { inner, table_ptr },
            }
        }
    }

    pub fn from_stack(ptr: *mut sys::lua_State, idx: i32) -> TableRef {
        unsafe {
            let lua = InnerLua::from_ptr(ptr);
            sys::lua_pushvalue(ptr, idx);
            let table_ptr = sys::lua_topointer(ptr, -1);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            let inner = Rc::new(OwnedInner { lua, id });
            Self {
                state: OwnedState { inner, table_ptr },
            }
        }
    }
}

impl Clone for TableRef {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

unsafe impl FromLua for StackTable {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_istable(ptr, idx) } != 0 {
            Some(StackTable::from_stack(ptr, idx))
        } else {
            None
        }
    }
}

unsafe impl FromLua for TableRef {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_istable(ptr, idx) } != 0 {
            Some(TableRef::from_stack(ptr, idx))
        } else {
            None
        }
    }
}

unsafe impl ToLua for &StackTable {
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        if ptr != self.state.ptr {
            panic!("unsafe cross-vm operation: Table belongs to a different Lua state");
        }
        unsafe { sys::lua_pushvalue(self.state.ptr, self.state.idx) };
    }
}

unsafe impl ToLua for StackTable {
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        (&self).to_lua(ptr);
    }
}

unsafe impl ToLua for &TableRef {
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        unsafe { self.state.inner.lua.push_ref(ptr, self.state.inner.id) };
    }
}

unsafe impl ToLua for TableRef {
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        (&self).to_lua(ptr);
    }
}

impl<M> IsType for Table<M>
where
    M: Mode + TableStorage,
    M::State: TableAccess,
{
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_istable(ptr, idx) != 0 }
    }
}

impl<M1: Mode, M2: Mode> PartialEq<Table<M2>> for Table<M1>
where
    M1: Mode + TableStorage,
    M1::State: TableAccess,
    M2: Mode + TableStorage,
    M2::State: TableAccess,
{
    fn eq(&self, other: &Table<M2>) -> bool {
        unsafe { self.state.get_table_ptr() == other.state.get_table_ptr() }
    }
}

impl<M: Mode> Eq for Table<M>
where
    M: Mode + TableStorage,
    M::State: TableAccess,
{
}

impl<M: Mode> Hash for Table<M>
where
    M: Mode + TableStorage,
    M::State: TableAccess,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.state.get_table_ptr().hash(state) }
    }
}

#[derive(Debug)]
pub struct Guard<'t>(*mut sys::lua_State, i32, TableView<'t>);

impl<'t> Deref for Guard<'t> {
    type Target = TableView<'t>;

    fn deref(&self) -> &Self::Target {
        &self.2
    }
}

impl<'t> Drop for Guard<'t> {
    fn drop(&mut self) {
        unsafe { sys::lua_settop(self.0, self.1) }
    }
}

#[derive(Debug)]
pub struct GuardMut<'t>(*mut sys::lua_State, i32, TableView<'t>);

impl<'t> Deref for GuardMut<'t> {
    type Target = TableView<'t>;

    fn deref(&self) -> &Self::Target {
        &self.2
    }
}

impl<'t> DerefMut for GuardMut<'t> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.2
    }
}

impl<'t> Drop for GuardMut<'t> {
    fn drop(&mut self) {
        unsafe { sys::lua_settop(self.0, self.1) }
    }
}
