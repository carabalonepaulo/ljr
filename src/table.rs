use std::{marker::PhantomData, rc::Rc};

use crate::{defer, from_lua::FromLua, to_lua::ToLua};
use luajit2_sys as sys;

#[derive(Debug)]
struct Inner {
    ptr: *mut sys::lua_State,
    id: i32,
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe { luajit2_sys::luaL_unref(self.ptr, luajit2_sys::LUA_REGISTRYINDEX, self.id) };
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
            unsafe { sys::lua_rawgeti(self.tref.0, self.tref.1, self.current) };
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
    pub fn set(&mut self, key: impl TableKey, value: impl ToLua) {
        key.to_lua(self.0);
        value.to_lua(self.0);
        unsafe { sys::lua_settable(self.0, self.1) };
    }

    pub fn get<T: FromLua>(&self, key: impl TableKey) -> Option<T::Output> {
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

        unsafe { sys::lua_rawgeti(self.0, self.1, len) };
        let val = <T as FromLua>::from_lua(self.0, -1);

        unsafe {
            sys::lua_pushnil(self.0);
            sys::lua_rawseti(self.0, self.1, len);
        }

        val
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
        let id = unsafe { luajit2_sys::luaL_ref(ptr, luajit2_sys::LUA_REGISTRYINDEX) };
        Self(Rc::new(Inner { ptr, id }))
    }

    pub(crate) fn from_stack(ptr: *mut sys::lua_State, idx: i32) -> Self {
        unsafe { sys::lua_pushvalue(ptr, idx) };
        let id = unsafe { luajit2_sys::luaL_ref(ptr, luajit2_sys::LUA_REGISTRYINDEX) };
        Self(Rc::new(Inner { ptr, id }))
    }

    pub(crate) fn id(&self) -> i32 {
        self.0.id
    }

    pub fn len(&self) -> usize {
        unsafe { sys::lua_rawgeti(self.0.ptr, sys::LUA_REGISTRYINDEX, self.0.id) };
        let len = unsafe { sys::lua_objlen(self.0.ptr, -1) };
        unsafe { sys::lua_pop(self.0.ptr, 1) };
        len
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TableRef) -> R,
    {
        let ptr = self.0.ptr;
        let id = self.0.id;

        defer!(pop, unsafe { sys::lua_pop(ptr, 1) });

        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, id) };
        let idx = unsafe { sys::lua_gettop(ptr) };
        let mut tref = TableRef(ptr, idx);
        f(&mut tref)
    }
}

pub trait TableKey: ToLua + FromLua {}

macro_rules! impl_table_key {
    ($($t:ty),*) => {
        $(impl TableKey for $t {})*
    }
}

impl_table_key!(i32, f32, f64, bool, String);
