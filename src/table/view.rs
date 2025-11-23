use std::marker::PhantomData;

use crate::{from_lua::FromLua, sys, table::constraints::TableKey, to_lua::ToLua};

#[derive(Debug)]
pub struct TableView<'t>(*mut sys::lua_State, i32, PhantomData<&'t ()>);

impl<'t> TableView<'t> {
    pub(crate) fn new(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self(ptr, idx, PhantomData)
    }

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

    pub fn ipairs<'s, T: FromLua>(&'s self) -> Ipairs<'t, 's, T> {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;
        Ipairs {
            tref: self,
            current: 1,
            len,
            marker: PhantomData,
        }
    }

    pub fn pairs<'s, K: FromLua, V: FromLua>(&'s self) -> Pairs<'t, 's, K, V> {
        Pairs {
            tref: self,
            started: false,
            finished: false,
            marker: PhantomData,
        }
    }
}

pub struct Ipairs<'t, 's, T> {
    tref: &'s TableView<'t>,
    current: i32,
    len: i32,
    marker: PhantomData<T>,
}

impl<'t, 's, T: FromLua> Iterator for Ipairs<'t, 's, T> {
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

pub struct Pairs<'t, 's, K, V> {
    tref: &'s TableView<'t>,
    started: bool,
    finished: bool,
    marker: PhantomData<(K, V)>,
}

impl<'t, 's, K: FromLua, V: FromLua> Iterator for Pairs<'t, 's, K, V> {
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
