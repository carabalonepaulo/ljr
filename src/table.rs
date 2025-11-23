use std::{collections::HashMap, marker::PhantomData, rc::Rc};

use crate::lstr::StrRef;
use crate::sys;
use crate::{defer, from_lua::FromLua, to_lua::ToLua};

#[derive(Debug)]
struct Inner {
    ptr: *mut sys::lua_State,
    id: i32,
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe { crate::sys::luaL_unref(self.ptr, crate::sys::LUA_REGISTRYINDEX, self.id as _) };
    }
}

pub struct Ipairs<'a, T> {
    tref: &'a mut TableRef,
    current: i32,
    len: i32,
    marker: PhantomData<T>,
}

impl<'a, T: FromLua> Iterator for Ipairs<'a, T> {
    type Item = (i32, T::Output);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current <= self.len {
            unsafe { sys::lua_rawgeti(self.tref.0, self.tref.1, self.current as _) };
            let val = <T as FromLua>::from_lua(self.tref.0, -1);
            unsafe { sys::lua_pop(self.tref.0, 1) };

            self.current += 1;

            if val.is_some() {
                return val.map(|v| (self.current - 1, v));
            }
        }

        None
    }
}

pub struct Pairs<'a, K, V> {
    tref: &'a mut TableRef,
    started: bool,
    finished: bool,
    marker: PhantomData<(K, V)>,
}

impl<'a, K: FromLua, V: FromLua> Iterator for Pairs<'a, K, V> {
    type Item = (K::Output, V::Output);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let ptr = self.tref.0;
        let idx = self.tref.1;

        unsafe {
            if !self.started {
                sys::lua_pushnil(ptr);
                self.started = true;
            }

            loop {
                if sys::lua_next(ptr, idx) == 0 {
                    self.finished = true;
                    return None;
                }

                let key = <K as FromLua>::from_lua(ptr, -2);
                let value = <V as FromLua>::from_lua(ptr, -1);

                sys::lua_pop(ptr, 1);

                if let (Some(k), Some(v)) = (key, value) {
                    return Some((k, v));
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TableRef(*mut sys::lua_State, i32);

impl TableRef {
    pub fn set<'a>(&mut self, key: impl TableKey<'a>, value: impl ToLua) {
        key.to_lua(self.0);
        value.to_lua(self.0);
        unsafe { sys::lua_settable(self.0, self.1) };
    }

    pub fn get<'a, T: FromLua>(&self, key: impl TableKey<'a>) -> Option<T::Output> {
        key.to_lua(self.0);
        unsafe { sys::lua_gettable(self.0, self.1) };
        let val = <T as FromLua>::from_lua(self.0, -1);
        unsafe { sys::lua_pop(self.0, 1) };
        val
    }

    pub fn push(&mut self, value: impl ToLua) {
        value.to_lua(self.0);
        let len = unsafe { sys::lua_objlen(self.0, self.1) };
        unsafe { sys::lua_rawseti(self.0, self.1, (len + 1) as _) };
    }

    pub fn pop<T: FromLua>(&mut self) -> Option<T::Output> {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;
        if len == 0 {
            return None;
        }

        unsafe { sys::lua_rawgeti(self.0, self.1, len as _) };
        let val = <T as FromLua>::from_lua(self.0, -1);

        unsafe {
            sys::lua_pushnil(self.0);
            sys::lua_rawseti(self.0, self.1, len as _);
        }

        val
    }

    pub fn clear(&mut self) {
        let ptr = self.0;
        let idx = self.1;

        unsafe {
            sys::lua_pushnil(ptr);
            while sys::lua_next(ptr, idx) != 0 {
                sys::lua_pop(ptr, 1);
                sys::lua_pushvalue(ptr, -1);
                sys::lua_pushnil(ptr);
                sys::lua_settable(ptr, idx);
            }
        }
    }

    pub fn len(&self) -> usize {
        unsafe { sys::lua_objlen(self.0, self.1) }
    }

    pub fn ipairs<'a, T: FromLua>(&'a mut self) -> Ipairs<'a, T> {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;
        Ipairs {
            tref: self,
            current: 1,
            len,
            marker: PhantomData,
        }
    }

    pub fn pairs<'a, K: FromLua, V: FromLua>(&'a mut self) -> Pairs<'a, K, V> {
        Pairs {
            tref: self,
            started: false,
            finished: false,
            marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct Table(Rc<Inner>);

impl Table {
    pub(crate) fn new(ptr: *mut sys::lua_State) -> Self {
        unsafe { sys::lua_newtable(ptr) };
        let id = unsafe { crate::sys::luaL_ref(ptr, crate::sys::LUA_REGISTRYINDEX) };
        Self(Rc::new(Inner { ptr, id }))
    }

    pub(crate) fn from_stack(ptr: *mut sys::lua_State, idx: i32) -> Self {
        unsafe { sys::lua_pushvalue(ptr, idx) };
        let id = unsafe { crate::sys::luaL_ref(ptr, crate::sys::LUA_REGISTRYINDEX) };
        Self(Rc::new(Inner { ptr, id }))
    }

    pub(crate) fn id(&self) -> i32 {
        self.0.id
    }

    pub fn len(&self) -> usize {
        unsafe { sys::lua_rawgeti(self.0.ptr, sys::LUA_REGISTRYINDEX, self.0.id as _) };
        let len = unsafe { sys::lua_objlen(self.0.ptr, -1) };
        unsafe { sys::lua_pop(self.0.ptr, 1) };
        len
    }

    pub fn clear(&mut self) {
        self.with(|t| t.clear());
    }

    pub fn extend_from_slice<T: ToLua + Clone>(&mut self, src: &[T]) {
        self.with(|t| {
            src.iter().for_each(|v| t.push(v.clone()));
        });
    }

    pub fn extend_from_map<'a, K: TableKey<'a> + Clone, V: FromLua + ToLua + Clone>(
        &mut self,
        src: &HashMap<K, V>,
    ) {
        self.with(|t| {
            src.iter().for_each(|(k, v)| t.set(k.clone(), v.clone()));
        });
    }

    pub fn with<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut TableRef) -> R,
    {
        let ptr = self.0.ptr;
        let id = self.0.id;

        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, id as _) };
        let top = unsafe { sys::lua_gettop(ptr) };
        defer!(pop, unsafe { sys::lua_settop(ptr, top - 1) });

        let mut tref = TableRef(ptr, top);
        f(&mut tref)
    }
}

pub trait TableKey<'a> {
    type Output;

    fn to_lua(self, ptr: *mut sys::lua_State);

    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self::Output>;
}

pub trait TableValue<'a> {
    type Output;

    fn to_lua(self, ptr: *mut sys::lua_State);

    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self::Output>;
}

macro_rules! impl_table_key {
    ($($ty:ty),*) => {
        $(
            impl<'a> TableKey<'a> for $ty {
                type Output = $ty;

                fn to_lua(self, ptr: *mut sys::lua_State) {
                    <Self as ToLua>::to_lua(self, ptr);
                }

                fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self::Output> {
                    <Self as FromLua>::from_lua(ptr, idx)
                }
            }
        )*
    };
}

impl_table_key!(i32, f32, f64, bool, String, StrRef);

impl<'a> TableKey<'a> for &str {
    type Output = String;

    fn to_lua(self, ptr: *mut crate::sys::lua_State) {
        <&str as ToLua>::to_lua(self, ptr);
    }

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        <String as FromLua>::from_lua(ptr, idx)
    }
}
