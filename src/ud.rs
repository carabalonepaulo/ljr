use std::{
    cell::{Ref, RefCell, RefMut},
    hash::{Hash, Hasher},
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    Borrowed, Mode, Owned, UserData, from_lua::FromLua, is_type::IsType, lua::InnerLua, sys,
    to_lua::ToLua,
};

pub type StackUd<T> = Ud<Borrowed, T>;

pub type UdRef<T> = Ud<Owned, T>;

pub struct OwnedUserData<M: Mode, T: UserData>(Rc<InnerLua>, i32, PhantomData<(M, T)>);

impl<M, T> Drop for OwnedUserData<M, T>
where
    M: Mode,
    T: UserData,
{
    fn drop(&mut self) {
        if let Some(ptr) = self.0.try_state() {
            unsafe { sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, self.1) }
        }
    }
}

pub enum Ud<M: Mode, T: UserData> {
    Borrowed(*mut sys::lua_State, i32, *mut *mut RefCell<T>),
    Owned(Rc<OwnedUserData<M, T>>, *mut *mut RefCell<T>),
}

impl<M, T> Ud<M, T>
where
    M: Mode,
    T: UserData,
{
    pub(crate) fn borrowed(ptr: *mut sys::lua_State, idx: i32) -> Self {
        let ud_ptr = unsafe { sys::lua_touserdata(ptr, idx) } as *mut *mut RefCell<T>;
        let abs_idx = unsafe { sys::lua_absindex(ptr, idx) };
        Self::Borrowed(ptr, abs_idx, ud_ptr)
    }

    pub(crate) fn owned(inner: Rc<InnerLua>, idx: i32) -> Self {
        unsafe {
            let ptr = inner.state();
            let idx = sys::lua_absindex(ptr, idx);
            let ud_ptr = sys::lua_touserdata(ptr, idx) as *mut *mut RefCell<T>;
            sys::lua_pushvalue(ptr, idx);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);
            Self::Owned(Rc::new(OwnedUserData(inner, id, PhantomData)), ud_ptr)
        }
    }

    pub fn to_owned(&self) -> Self {
        match self {
            Ud::Borrowed(ptr, idx, _) => Self::owned(InnerLua::from_ptr(*ptr), *idx),
            Ud::Owned(ud, ud_ptr) => Self::Owned(ud.clone(), *ud_ptr),
        }
    }

    pub fn as_ref(&self) -> Ref<'_, T> {
        match self {
            Ud::Borrowed(_, _, ud_ptr) => unsafe {
                let cell: &RefCell<T> = &***ud_ptr;
                cell.borrow()
            },
            Ud::Owned(ud, ud_ptr) => unsafe {
                let _ = ud.0.state();
                let cell: &RefCell<T> = &***ud_ptr;
                cell.borrow()
            },
        }
    }

    pub fn as_mut(&mut self) -> RefMut<'_, T> {
        match self {
            Ud::Borrowed(_, _, ud_ptr) => unsafe {
                let cell: &RefCell<T> = &mut ***ud_ptr;
                cell.borrow_mut()
            },
            Ud::Owned(ud, ud_ptr) => unsafe {
                let _ = ud.0.state();
                let cell: &RefCell<T> = &mut ***ud_ptr;
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

    fn internal_ptr(&self) -> *mut *mut RefCell<T> {
        match self {
            Ud::Borrowed(_, _, ud_ptr) => *ud_ptr,
            Ud::Owned(_, ud_ptr) => *ud_ptr,
        }
    }
}

impl<M, T> Clone for Ud<M, T>
where
    M: Mode,
    T: UserData,
{
    fn clone(&self) -> Self {
        Self::to_owned(self)
    }
}

unsafe impl<T> FromLua for StackUd<T>
where
    T: UserData,
{
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        unsafe {
            sys::lua_pushvalue(ptr, idx);

            if sys::lua_getmetatable(ptr, -1) == 0 {
                sys::lua_pop(ptr, 1);
                return None;
            }

            sys::lua_rawgeti(ptr, -1, 1);
            let type_id = sys::lua_tolightuserdata(ptr, -1);
            sys::lua_pop(ptr, 1);

            let expected_type_id = T::functions().as_ptr() as *mut std::ffi::c_void;
            if type_id != expected_type_id {
                sys::lua_pop(ptr, 2);
                return None;
            }

            sys::lua_pop(ptr, 2);
        }

        Some(Ud::<Borrowed, T>::borrowed(ptr, idx))
    }
}

unsafe impl<T> FromLua for UdRef<T>
where
    T: UserData,
{
    fn from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Option<Self> {
        unsafe {
            sys::lua_pushvalue(ptr, idx);

            if sys::lua_getmetatable(ptr, -1) == 0 {
                sys::lua_pop(ptr, 1);
                return None;
            }

            sys::lua_rawgeti(ptr, -1, 1);
            let type_id = sys::lua_tolightuserdata(ptr, -1);
            sys::lua_pop(ptr, 1);

            let expected_type_id = T::functions().as_ptr() as *mut std::ffi::c_void;
            if type_id != expected_type_id {
                sys::lua_pop(ptr, 2);
                return None;
            }

            sys::lua_pop(ptr, 1);
            let value = Some(Self::owned(InnerLua::from_ptr(ptr), idx));
            sys::lua_pop(ptr, 1);
            value
        }
    }
}

unsafe impl<M, T> ToLua for &Ud<M, T>
where
    M: Mode,
    T: UserData,
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        unsafe {
            match self {
                Ud::Borrowed(_, idx, _) => sys::lua_pushvalue(ptr, *idx),
                Ud::Owned(ud, _) => {
                    sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, ud.1 as _);
                }
            }
        }
    }
}

unsafe impl<M, T> ToLua for Ud<M, T>
where
    M: Mode,
    T: UserData,
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        unsafe {
            match self {
                Ud::Borrowed(_, idx, _) => sys::lua_pushvalue(ptr, idx),
                Ud::Owned(ud, _) => ud.0.push_ref(ptr, ud.1),
            }
        }
    }
}

impl<M1: Mode, M2: Mode, T: UserData> PartialEq<Ud<M2, T>> for Ud<M1, T> {
    fn eq(&self, other: &Ud<M2, T>) -> bool {
        self.internal_ptr() == other.internal_ptr()
    }
}

impl<M: Mode, T: UserData> Eq for Ud<M, T> {}

impl<M: Mode, T: UserData> Hash for Ud<M, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.internal_ptr().hash(state);
    }
}

impl<M, T> IsType for Ud<M, T>
where
    M: Mode,
    T: UserData,
{
    fn is_type(ptr: *mut crate::sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_isuserdata(ptr, idx) != 0 }
    }
}
