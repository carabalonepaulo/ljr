use std::{collections::HashMap, marker::PhantomData};

use crate::{
    Nil, error::Error, from_lua::FromLua, is_type::IsType, lua::ValueArg, sys, to_lua::ToLua,
};

const SHARED_REMOVE_KEY: usize = 0x6C6A72_01;
const SHARED_INSERT_KEY: usize = 0x6C6A72_02;

unsafe fn ensure_cached_func(
    ptr: *mut sys::lua_State,
    key: *mut std::ffi::c_void,
    name: &std::ffi::CStr,
) {
    unsafe {
        sys::lua_pushlightuserdata(ptr, key);
        sys::lua_rawget(ptr, sys::LUA_REGISTRYINDEX);

        if sys::lua_isfunction(ptr, -1) == 0 {
            sys::lua_pop(ptr, 1);

            sys::lua_getglobal(ptr, c"table".as_ptr());
            if sys::lua_istable(ptr, -1) != 0 {
                sys::lua_getfield(ptr, -1, name.as_ptr());
                sys::lua_remove(ptr, -2);

                if sys::lua_isfunction(ptr, -1) != 0 {
                    sys::lua_pushlightuserdata(ptr, key);
                    sys::lua_pushvalue(ptr, -2);
                    sys::lua_rawset(ptr, sys::LUA_REGISTRYINDEX);
                    return;
                }
                sys::lua_pop(ptr, 1);
            } else {
                sys::lua_pop(ptr, 1);
            }
            sys::lua_pushnil(ptr);
        }
    }
}

#[derive(Debug)]
pub struct TableView<'t>(*mut sys::lua_State, i32, PhantomData<&'t ()>);

impl<'t> TableView<'t> {
    pub(crate) fn new(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self(ptr, idx, PhantomData)
    }

    pub fn set<'a, K: ToLua, V: ToLua>(&mut self, key: K, value: V) {
        const { assert!(K::LEN == 1 && V::LEN == 1) }
        key.to_lua(self.0);
        value.to_lua(self.0);
        unsafe { sys::lua_settable(self.0, self.1) };
    }

    pub fn get<'a, K: ToLua, V: FromLua + ValueArg>(&self, key: K) -> Option<V> {
        const { assert!(K::LEN == 1 && V::LEN == 1) }
        key.to_lua(self.0);
        unsafe { sys::lua_gettable(self.0, self.1) };
        let val = V::from_lua(self.0, -1);
        unsafe { sys::lua_pop(self.0, 1) };
        val
    }

    pub fn view<'a, K: ToLua, V: FromLua, F: FnOnce(&V) -> R, R>(&self, key: K, f: F) -> Option<R> {
        const { assert!(K::LEN == 1 && V::LEN == 1) }
        key.to_lua(self.0);
        unsafe { sys::lua_gettable(self.0, self.1) };
        let value = V::from_lua(self.0, -1);
        let result = if let Some(value) = value {
            Some(f(&value))
        } else {
            None
        };
        unsafe { sys::lua_pop(self.0, 1) };
        result
    }

    pub fn push<T: ToLua>(&mut self, value: T) {
        const { assert!(T::LEN == 1) }
        value.to_lua(self.0);
        let len = unsafe { sys::lua_objlen(self.0, self.1) };
        unsafe { sys::lua_rawseti(self.0, self.1, (len + 1) as _) };
    }

    pub fn pop<T: FromLua + ValueArg>(&mut self) -> Option<T> {
        const { assert!(T::LEN == 1) }
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
        const { assert!(T::LEN == 1) }
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
        self.try_insert(index, value)
            .unwrap_or_else(|e| panic!("{}", e))
    }

    pub fn try_insert(&mut self, index: i32, value: impl ToLua) -> Result<(), Error> {
        unsafe {
            let ptr = self.0;
            let t_idx = self.1;
            let key_addr = SHARED_INSERT_KEY as *mut std::ffi::c_void;

            ensure_cached_func(ptr, key_addr, c"insert");

            if Nil::is_type(self.0, -1) {
                sys::lua_pop(self.0, 1);
                return Err(Error::MissingGlobal("table.insert".into()));
            }

            sys::lua_pushvalue(ptr, t_idx);
            sys::lua_pushinteger(ptr, index as _);
            value.to_lua_unchecked(ptr);

            if sys::lua_pcall(ptr, 3, 0, 0) != 0 {
                let err = Error::from_stack(ptr, -1);
                sys::lua_pop(ptr, 1);
                Err(err)
            } else {
                Ok(())
            }
        }
    }

    pub fn remove_then<T: FromLua + IsType, F: FnOnce(&T) -> R, R>(
        &mut self,
        index: i32,
        f: F,
    ) -> Result<R, Error> {
        unsafe {
            self.remove_impl::<T>(index)?;

            let value = <T as FromLua>::from_lua(self.0, -1);
            let result = if let Some(value) = value {
                Some(f(&value))
            } else {
                None
            };

            sys::lua_pop(self.0, 1);
            result.ok_or_else(|| Error::WrongReturnType)
        }
    }

    pub fn remove<T: FromLua + ValueArg + IsType>(&mut self, index: i32) -> Result<T, Error> {
        const { assert!(T::LEN == 1) }
        unsafe {
            self.remove_impl::<T>(index)?;

            let val = <T as FromLua>::from_lua(self.0, -1);
            sys::lua_pop(self.0, 1);
            val.ok_or_else(|| Error::WrongReturnType)
        }
    }

    unsafe fn remove_impl<T: IsType>(&self, idx: i32) -> Result<(), Error> {
        unsafe {
            let key_addr = SHARED_REMOVE_KEY as *mut std::ffi::c_void;

            sys::lua_rawgeti(self.0, self.1, idx as _);
            if !<T as IsType>::is_type(self.0, -1) {
                sys::lua_pop(self.0, 1);
                return Err(Error::WrongReturnType);
            }
            sys::lua_pop(self.0, 1);

            ensure_cached_func(self.0, key_addr, c"remove");

            if Nil::is_type(self.0, -1) {
                sys::lua_pop(self.0, 1);
                return Err(Error::MissingGlobal("table.remove".into()));
            }

            sys::lua_pushvalue(self.0, self.1);
            sys::lua_pushinteger(self.0, idx as _);

            if sys::lua_pcall(self.0, 2, 1, 0) != 0 {
                let err = Error::from_stack(self.0, -1);
                sys::lua_pop(self.0, 1);
                Err(err)
            } else {
                Ok(())
            }
        }
    }

    pub fn contains_key<'a, K: ToLua>(&self, key: K) -> bool {
        const { assert!(K::LEN == 1) }
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
        const { assert!(K::LEN == 1 && V::LEN == 1) }
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
        const { assert!(T::LEN == 1) }
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
        const { assert!(K::LEN == 1 && V::LEN == 1) }
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
