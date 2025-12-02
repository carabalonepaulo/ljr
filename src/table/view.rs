use std::{collections::HashMap, marker::PhantomData};

use crate::{from_lua::FromLua, lua::ValueArg, sys, to_lua::ToLua};

#[derive(Debug)]
pub struct TableView<'t>(*mut sys::lua_State, i32, PhantomData<&'t ()>);

impl<'t> TableView<'t> {
    pub(crate) fn new(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self(ptr, idx, PhantomData)
    }

    pub fn set<'a>(&mut self, key: impl ToLua, value: impl ToLua) {
        key.to_lua(self.0);
        value.to_lua(self.0);
        unsafe { sys::lua_settable(self.0, self.1) };
    }

    pub fn get<'a, T: FromLua + ValueArg>(&self, key: impl ToLua) -> Option<T> {
        key.to_lua(self.0);
        unsafe { sys::lua_gettable(self.0, self.1) };
        let val = <T as FromLua>::from_lua(self.0, -1);
        unsafe { sys::lua_pop(self.0, 1) };
        val
    }

    pub fn view<'a, T: FromLua, F: FnOnce(&T) -> R, R>(&self, key: impl ToLua, f: F) -> Option<R> {
        key.to_lua(self.0);
        unsafe { sys::lua_gettable(self.0, self.1) };
        let value = <T as FromLua>::from_lua(self.0, -1);
        let result = if let Some(value) = value {
            Some(f(&value))
        } else {
            None
        };
        unsafe { sys::lua_pop(self.0, 1) };
        result
    }

    pub fn push(&mut self, value: impl ToLua) {
        value.to_lua(self.0);
        let len = unsafe { sys::lua_objlen(self.0, self.1) };
        unsafe { sys::lua_rawseti(self.0, self.1, (len + 1) as _) };
    }

    pub fn pop<T: FromLua + ValueArg>(&mut self) -> Option<T> {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;
        if len == 0 {
            return None;
        }

        unsafe { sys::lua_rawgeti(self.0, self.1, len as _) };
        let val = <T as FromLua>::from_lua(self.0, -1);
        unsafe { sys::lua_pop(self.0, 1) };

        unsafe {
            sys::lua_pushnil(self.0);
            sys::lua_rawseti(self.0, self.1, len as _);
        }

        val
    }

    pub fn pop_then<'a, T: FromLua, F: FnOnce(&T) -> R, R>(&self, f: F) -> Option<R> {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;
        if len == 0 {
            return None;
        }

        unsafe { sys::lua_rawgeti(self.0, self.1, len as _) };
        let value = <T as FromLua>::from_lua(self.0, -1);
        let result = if let Some(value) = value {
            let result = f(&value);
            unsafe {
                sys::lua_pushnil(self.0);
                sys::lua_rawseti(self.0, self.1, len as _);
            }

            Some(result)
        } else {
            None
        };

        unsafe { sys::lua_pop(self.0, 1) };
        result
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, index: i32, value: impl ToLua) {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;

        let effective_idx = if index < 1 { 1 } else { index };
        let effective_idx = if effective_idx > len + 1 {
            len + 1
        } else {
            effective_idx
        };

        for i in (effective_idx..=len).rev() {
            unsafe {
                sys::lua_rawgeti(self.0, self.1, i as _);
                sys::lua_rawseti(self.0, self.1, (i + 1) as _);
            }
        }

        value.to_lua(self.0);
        unsafe { sys::lua_rawseti(self.0, self.1, effective_idx as _) };
    }

    pub fn remove<T: FromLua + ValueArg>(&mut self, index: i32) -> Option<T> {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;

        if index < 1 || index > len {
            return None;
        }

        unsafe { sys::lua_rawgeti(self.0, self.1, index as _) };
        let value = <T as FromLua>::from_lua(self.0, -1);
        unsafe { sys::lua_pop(self.0, 1) };

        if value.is_some() {
            for i in index..len {
                unsafe {
                    sys::lua_rawgeti(self.0, self.1, (i + 1) as _);
                    sys::lua_rawseti(self.0, self.1, i as _);
                }
            }

            unsafe {
                sys::lua_pushnil(self.0);
                sys::lua_rawseti(self.0, self.1, len as _);
            }
        }

        value
    }

    pub fn remove_then<T: FromLua, F: FnOnce(&T) -> R, R>(
        &mut self,
        index: i32,
        f: F,
    ) -> Option<R> {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;

        if index < 1 || index > len {
            return None;
        }

        unsafe { sys::lua_rawgeti(self.0, self.1, index as _) };
        let value = <T as FromLua>::from_lua(self.0, -1);
        let result = if let Some(value) = value {
            let result = f(&value);
            Some(result)
        } else {
            None
        };
        unsafe { sys::lua_pop(self.0, 1) };

        if result.is_some() {
            for i in index..len {
                unsafe {
                    sys::lua_rawgeti(self.0, self.1, (i + 1) as _);
                    sys::lua_rawseti(self.0, self.1, i as _);
                }
            }

            unsafe {
                sys::lua_pushnil(self.0);
                sys::lua_rawseti(self.0, self.1, len as _);
            }
        }

        result
    }

    pub fn contains_key<'a>(&self, key: impl ToLua) -> bool {
        key.to_lua(self.0);
        unsafe { sys::lua_gettable(self.0, self.1) };
        let not_nil = unsafe { sys::lua_isnil(self.0, -1) == 0 };
        unsafe { sys::lua_pop(self.0, 1) };
        not_nil
    }

    pub fn for_each<K, V, F>(&self, mut f: F)
    where
        K: FromLua,
        V: FromLua,
        F: FnMut(&K, &V) -> bool,
    {
        let ptr = self.0;
        let t_idx = self.1;

        unsafe {
            sys::lua_pushnil(ptr);

            while sys::lua_next(ptr, t_idx) != 0 {
                let key = <K as FromLua>::from_lua(ptr, -2);
                let val = <V as FromLua>::from_lua(ptr, -1);

                let should_continue = match (key, val) {
                    (Some(k), Some(v)) => f(&k, &v),
                    _ => true,
                };

                sys::lua_pop(ptr, 1);

                if !should_continue {
                    sys::lua_pop(ptr, 1);
                    return;
                }
            }
        }
    }

    pub fn ipairs<'s, T: FromLua + ValueArg>(&'s self) -> Ipairs<'t, 's, T> {
        let len = unsafe { sys::lua_objlen(self.0, self.1) } as i32;
        Ipairs {
            tref: self,
            current: 1,
            len,
            marker: PhantomData,
        }
    }

    pub fn pairs<'s, K: FromLua + ValueArg, V: FromLua + ValueArg>(
        &'s self,
    ) -> Pairs<'t, 's, K, V> {
        Pairs {
            tref: self,
            started: false,
            finished: false,
            marker: PhantomData,
        }
    }

    pub fn extend_from_slice<T: ToLua + Clone>(&mut self, src: &[T]) {
        src.iter().for_each(|v| self.push(v.clone()));
    }

    pub fn extend_from_map<'a, K: ToLua + Clone, V: FromLua + ToLua + Clone>(
        &mut self,
        src: &HashMap<K, V>,
    ) {
        src.iter().for_each(|(k, v)| self.set(k.clone(), v.clone()));
    }
}

pub struct Ipairs<'t, 's, T: FromLua + ValueArg> {
    tref: &'s TableView<'t>,
    current: i32,
    len: i32,
    marker: PhantomData<T>,
}

impl<'t, 's, T> Iterator for Ipairs<'t, 's, T>
where
    T: FromLua + ValueArg,
{
    type Item = (i32, T);

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

impl<'t, 's, K, V> Iterator for Pairs<'t, 's, K, V>
where
    K: FromLua + ValueArg,
    V: FromLua + ValueArg,
{
    type Item = (K, V);

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

impl<'t, 's, K, V> Drop for Pairs<'t, 's, K, V> {
    fn drop(&mut self) {
        if self.started && !self.finished {
            unsafe { sys::lua_pop(self.tref.0, 1) };
        }
    }
}
