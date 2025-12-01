pub mod builder;
pub mod constraints;
pub mod view;

use std::{
    cmp::{Eq, PartialEq},
    collections::HashMap,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{
    Borrowed, Mode, Owned,
    from_lua::FromLua,
    is_type::IsType,
    lua::InnerLua,
    sys,
    table::{constraints::TableKey, view::TableView},
    to_lua::ToLua,
};

pub type StackTable = Table<Borrowed>;

pub type TableRef = Table<Owned>;

#[derive(Debug)]
pub struct OwnedTable<M: Mode>(Rc<InnerLua>, i32, PhantomData<M>);

impl<M> Drop for OwnedTable<M>
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
pub enum Table<M: Mode> {
    Borrowed(*mut sys::lua_State, i32, *const std::ffi::c_void),
    Owned(Rc<OwnedTable<M>>, *const std::ffi::c_void),
}

impl<M> Table<M>
where
    M: Mode,
{
    pub(crate) fn new(inner_lua: Rc<InnerLua>) -> Self {
        unsafe {
            let ptr = inner_lua.state();
            sys::lua_newtable(ptr);
            let table_ptr = sys::lua_topointer(ptr, -1);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            Self::Owned(Rc::new(OwnedTable(inner_lua, id, PhantomData)), table_ptr)
        }
    }

    pub(crate) fn borrowed(ptr: *mut sys::lua_State, idx: i32) -> Self {
        unsafe {
            let idx = sys::lua_absindex(ptr, idx);
            let table_ptr = sys::lua_topointer(ptr, idx);
            Self::Borrowed(ptr, idx, table_ptr)
        }
    }

    pub(crate) fn owned(ptr: *mut sys::lua_State, idx: i32) -> TableRef {
        unsafe {
            let inner = InnerLua::from_ptr(ptr);
            sys::lua_pushvalue(ptr, idx);
            let table_ptr = sys::lua_topointer(ptr, -1);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            Table::<Owned>::Owned(Rc::new(OwnedTable(inner, id, PhantomData)), table_ptr)
        }
    }

    pub fn to_owned(&self) -> TableRef {
        match self {
            Table::Borrowed(ptr, idx, _) => Self::owned(*ptr, *idx),
            Table::Owned(inner, tptr) => {
                Table::<Owned>::Owned(unsafe { std::mem::transmute(inner.clone()) }, *tptr)
            }
        }
    }

    pub fn as_ref<'t>(&'t self) -> Guard<'t> {
        let ptr = match self {
            Table::Borrowed(ptr, idx, _) => unsafe {
                sys::lua_pushvalue(*ptr, *idx);
                *ptr
            },
            Table::Owned(inner, _) => unsafe {
                let ptr = inner.0.state();
                sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _);
                ptr
            },
        };

        let top = unsafe { sys::lua_gettop(ptr) };
        let t_idx = unsafe { sys::lua_absindex(ptr, -1) };
        let view = TableView::new(ptr, t_idx);
        Guard(ptr, top - 1, view)
    }

    pub fn as_mut<'t>(&'t mut self) -> GuardMut<'t> {
        let ptr = match self {
            Table::Borrowed(ptr, idx, _) => unsafe {
                sys::lua_pushvalue(*ptr, *idx);
                *ptr
            },
            Table::Owned(inner, _) => unsafe {
                let ptr = inner.0.state();
                sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _);
                ptr
            },
        };

        let top = unsafe { sys::lua_gettop(ptr) };
        let t_idx = unsafe { sys::lua_absindex(ptr, -1) };
        let view = TableView::new(ptr, t_idx);
        GuardMut(ptr, top - 1, view)
    }

    pub fn with<F: FnOnce(&TableView) -> R, R>(&self, f: F) -> R {
        let guard = self.as_ref();
        f(&*guard)
    }

    pub fn with_mut<F: FnOnce(&mut TableView) -> R, R>(&mut self, f: F) -> R {
        let mut guard = self.as_mut();
        f(&mut *guard)
    }

    pub fn len(&self) -> usize {
        let ptr = match self {
            Table::Borrowed(ptr, idx, _) => unsafe {
                sys::lua_pushvalue(*ptr, *idx);
                *ptr
            },
            Table::Owned(inner, _) => unsafe {
                let ptr = inner.0.state();
                sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _);
                ptr
            },
        };
        let len = unsafe { sys::lua_objlen(ptr, -1) };
        unsafe { sys::lua_pop(ptr, 1) };
        len
    }

    pub fn clear(&mut self) {
        self.with_mut(|t| t.clear());
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

    fn internal_ptr(&self) -> *const std::ffi::c_void {
        match self {
            Self::Borrowed(_, _, tptr) => *tptr,
            Self::Owned(_, tptr) => *tptr,
        }
    }
}

impl Clone for TableRef {
    fn clone(&self) -> Self {
        self.to_owned()
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

unsafe impl FromLua for StackTable {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        if unsafe { sys::lua_istable(ptr, idx) } != 0 {
            Some(Table::borrowed(ptr, idx))
        } else {
            None
        }
    }
}

unsafe impl FromLua for TableRef {
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<TableRef> {
        if unsafe { sys::lua_istable(ptr, idx) } != 0 {
            Some(Self::owned(ptr, idx))
        } else {
            None
        }
    }
}

unsafe impl<M> ToLua for &Table<M>
where
    M: Mode,
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        match self {
            Table::Borrowed(_, idx, _) => unsafe { sys::lua_pushvalue(ptr, *idx) },
            Table::Owned(inner, _) => unsafe {
                sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, inner.1 as _);
            },
        }
    }
}

unsafe impl<M> ToLua for Table<M>
where
    M: Mode,
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        unsafe {
            match self {
                Table::Borrowed(_, idx, _) => sys::lua_pushvalue(ptr, idx),
                Table::Owned(inner, _) => inner.0.push_ref(ptr, inner.1),
            }
        }
    }
}

impl<M> IsType for Table<M>
where
    M: Mode,
{
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_istable(ptr, idx) != 0 }
    }
}

impl<M1: Mode, M2: Mode> PartialEq<Table<M2>> for Table<M1> {
    fn eq(&self, other: &Table<M2>) -> bool {
        self.internal_ptr() == other.internal_ptr()
    }
}

impl<M: Mode> Eq for Table<M> {}

impl<M: Mode> Hash for Table<M> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.internal_ptr().hash(state);
    }
}
