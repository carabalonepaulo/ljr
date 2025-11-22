use std::{
    cell::{Ref, RefCell, RefMut},
    ffi::CStr,
    marker::PhantomData,
    rc::Rc,
};

use crate::{UserData, from_lua::FromLua, sys, to_lua::ToLua};

pub struct OwnedUserData<T: UserData>(*mut sys::lua_State, i32, PhantomData<T>);

impl<T> Drop for OwnedUserData<T>
where
    T: UserData,
{
    fn drop(&mut self) {
        unsafe { sys::luaL_unref(self.0, sys::LUA_REGISTRYINDEX, self.1) }
    }
}

pub enum Ud<T: UserData> {
    Borrowed(*mut sys::lua_State, i32),
    Owned(Rc<OwnedUserData<T>>),
}

impl<T> Ud<T>
where
    T: UserData,
{
    pub fn borrowed(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self::Borrowed(ptr, unsafe { sys::lua_absindex(ptr, idx) })
    }

    pub fn owned(ptr: *mut sys::lua_State, idx: i32) -> Self {
        unsafe {
            let idx = sys::lua_absindex(ptr, idx);
            sys::lua_pushvalue(ptr, idx);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            Self::Owned(Rc::new(OwnedUserData(ptr, id, PhantomData)))
        }
    }

    pub fn to_owned(&self) -> Self {
        match self {
            Ud::Borrowed(ptr, idx) => Self::owned(*ptr, *idx),
            Ud::Owned(ud) => Self::Owned(ud.clone()),
        }
    }

    pub fn as_ref(&self) -> Ref<'_, T> {
        match self {
            Ud::Borrowed(ptr, idx) => unsafe {
                let ud_ptr = sys::lua_touserdata(*ptr, *idx) as *const *const RefCell<T>;
                let cell: &RefCell<T> = &**ud_ptr;
                cell.borrow()
            },
            Ud::Owned(ud) => unsafe {
                let (ptr, id) = (ud.0, ud.1);

                sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, id as _);
                let ud_ptr = sys::lua_touserdata(ptr, -1) as *const *const RefCell<T>;
                sys::lua_pop(ptr, 1);

                let cell: &RefCell<T> = &**ud_ptr;
                cell.borrow()
            },
        }
    }

    pub fn as_mut(&mut self) -> RefMut<'_, T> {
        match self {
            Ud::Borrowed(ptr, idx) => unsafe {
                let ud_ptr = sys::lua_touserdata(*ptr, *idx) as *mut *mut RefCell<T>;
                let cell: &RefCell<T> = &mut **ud_ptr;
                cell.borrow_mut()
            },
            Ud::Owned(ud) => unsafe {
                let (ptr, id) = (ud.0, ud.1);

                sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, id as _);
                let ud_ptr = sys::lua_touserdata(ptr, -1) as *mut *mut RefCell<T>;
                sys::lua_pop(ptr, 1);

                let cell: &RefCell<T> = &mut **ud_ptr;
                cell.borrow_mut()
            },
        }
    }

    pub fn with<F: FnOnce(&T) -> R, R>(&self, f: F) -> R {
        let guard = self.as_ref();
        f(&*guard)
    }

    pub fn with_mut<F: FnOnce(&mut T) -> R, R>(&mut self, f: F) -> R {
        let mut guard = self.as_mut();
        f(&mut *guard)
    }
}

impl<T> Clone for Ud<T>
where
    T: UserData,
{
    fn clone(&self) -> Self {
        Self::to_owned(self)
    }
}

impl<T> FromLua for T
where
    T: UserData,
{
    type Output = Ud<T>;

    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self::Output> {
        unsafe {
            sys::lua_pushvalue(ptr, idx);

            if sys::lua_getmetatable(ptr, -1) == 0 {
                sys::lua_pop(ptr, 1);
                return None;
            }

            sys::lua_getfield(ptr, -1, c"__name".as_ptr());
            let mt_name = sys::lua_tostring(ptr, -1);

            let mt = CStr::from_ptr(mt_name);
            let expected = CStr::from_ptr(T::name());
            if mt != expected {
                sys::lua_pop(ptr, 3);
                return None;
            }

            sys::lua_pop(ptr, 2);
        }

        Some(Ud::<T>::borrowed(ptr, idx))
    }
}

impl<T> FromLua for Ud<T>
where
    T: UserData,
{
    type Output = Ud<T>;

    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self::Output> {
        unsafe {
            sys::lua_pushvalue(ptr, idx);

            if sys::lua_getmetatable(ptr, -1) == 0 {
                sys::lua_pop(ptr, 1);
                return None;
            }

            sys::lua_getfield(ptr, -1, c"__name".as_ptr());
            let mt_name = sys::lua_tostring(ptr, -1);

            let mt = CStr::from_ptr(mt_name);
            let expected = CStr::from_ptr(T::name());
            if mt != expected {
                sys::lua_pop(ptr, 3);
                return None;
            }

            sys::lua_pop(ptr, 2);

            let value = Some(Self::owned(ptr, idx));
            sys::lua_pop(ptr, 1);
            value
        }
    }
}

impl<T> ToLua for Ud<T>
where
    T: UserData,
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        unsafe {
            match self {
                Ud::Borrowed(_, idx) => sys::lua_pushvalue(ptr, idx),
                Ud::Owned(ud) => {
                    sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, ud.1 as _);
                }
            }
        }
    }
}
