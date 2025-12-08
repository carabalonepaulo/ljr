use std::{
    cell::{Ref, RefCell, RefMut},
    hash::{Hash, Hasher},
    rc::Rc,
};

use crate::{
    Borrowed, Mode, Owned, UserData,
    error::{Error, UnwrapDisplay},
    from_lua::FromLua,
    helper,
    is_type::IsType,
    lua::InnerLua,
    owned_value::LuaInnerHandle,
    prelude::OwnedValue,
    sys,
    to_lua::ToLua,
};

pub trait UserDataState<T> {
    type State;
}

pub trait UserDataAccess<T> {
    fn ud_ptr(&self) -> *mut *mut RefCell<T>;

    fn try_as_ref(&self) -> Result<Ref<'_, T>, Error>;

    fn as_ref(&self) -> Ref<'_, T> {
        self.try_as_ref().unwrap_display()
    }

    fn try_as_mut(&self) -> Result<RefMut<'_, T>, Error>;

    fn as_mut(&self) -> RefMut<'_, T> {
        self.try_as_mut().unwrap_display()
    }

    fn try_with<F: FnOnce(&T) -> R, R>(&self, f: F) -> Result<R, Error> {
        let guard = self.try_as_ref()?;
        Ok(f(&*guard))
    }

    fn with<F: FnOnce(&T) -> R, R>(&self, f: F) -> R {
        let guard = self.as_ref();
        f(&*guard)
    }

    fn try_with_mut<F: FnOnce(&mut T) -> R, R>(&mut self, f: F) -> Result<R, Error> {
        let mut guard = self.try_as_mut()?;
        Ok(f(&mut *guard))
    }

    fn with_mut<F: FnOnce(&mut T) -> R, R>(&mut self, f: F) -> R {
        let mut guard = self.as_mut();
        f(&mut *guard)
    }
}

pub struct BorrowedState<T>
where
    T: UserData,
{
    ptr: *mut sys::lua_State,
    idx: i32,
    ud_ptr: *mut *mut RefCell<T>,
}

impl<T> UserDataState<T> for Borrowed
where
    T: UserData,
{
    type State = BorrowedState<T>;
}

impl<T> UserDataAccess<T> for BorrowedState<T>
where
    T: UserData,
{
    fn ud_ptr(&self) -> *mut *mut RefCell<T> {
        self.ud_ptr
    }

    fn try_as_ref(&self) -> Result<Ref<'_, T>, Error> {
        Ok(unsafe { (&**self.ud_ptr).try_borrow()? })
    }

    fn try_as_mut(&self) -> Result<RefMut<'_, T>, Error> {
        Ok(unsafe { (&**self.ud_ptr).try_borrow_mut()? })
    }
}

#[derive(Debug)]
pub struct OwnedState<T>
where
    T: UserData,
{
    lua: RefCell<Rc<InnerLua>>,
    id: i32,
    ud_ptr: *mut *mut RefCell<T>,
}

impl<T> Drop for OwnedState<T>
where
    T: UserData,
{
    fn drop(&mut self) {
        if let Ok(ptr) = self.lua.borrow().try_state() {
            unsafe { sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, self.id) };
        }
    }
}

impl<T> UserDataState<T> for Owned
where
    T: UserData,
{
    type State = OwnedState<T>;
}

impl<T> UserDataAccess<T> for OwnedState<T>
where
    T: UserData,
{
    fn ud_ptr(&self) -> *mut *mut RefCell<T> {
        self.ud_ptr
    }

    fn try_as_ref(&self) -> Result<Ref<'_, T>, Error> {
        let _ = self.lua.borrow().try_state()?;
        Ok(unsafe { (&**self.ud_ptr).try_borrow()? })
    }

    fn try_as_mut(&self) -> Result<RefMut<'_, T>, Error> {
        let _ = self.lua.borrow().try_state()?;
        Ok(unsafe { (&**self.ud_ptr).try_borrow_mut()? })
    }
}

pub type StackUd<T> = Ud<Borrowed, T>;
pub type UdRef<T> = Ud<Owned, T>;

pub struct Ud<M, T>
where
    M: Mode + UserDataState<T>,
    M::State: UserDataAccess<T>,
{
    state: M::State,
}

impl<M, T> Ud<M, T>
where
    M: Mode + UserDataState<T>,
    M::State: UserDataAccess<T>,
{
    #[inline]
    pub fn try_as_ref(&self) -> Result<Ref<'_, T>, Error> {
        self.state.try_as_ref()
    }

    #[inline]
    pub fn as_ref(&self) -> Ref<'_, T> {
        self.state.as_ref()
    }

    #[inline]
    pub fn try_as_mut(&self) -> Result<RefMut<'_, T>, Error> {
        self.state.try_as_mut()
    }

    #[inline]
    pub fn as_mut(&self) -> RefMut<'_, T> {
        self.state.as_mut()
    }

    #[inline]
    pub fn try_with<F: FnOnce(&T) -> R, R>(&self, f: F) -> Result<R, Error> {
        self.state.try_with(f)
    }

    #[inline]
    pub fn with<F: FnOnce(&T) -> R, R>(&self, f: F) -> R {
        self.state.with(f)
    }

    #[inline]
    pub fn try_with_mut<F: FnOnce(&mut T) -> R, R>(&mut self, f: F) -> Result<R, Error> {
        self.state.try_with_mut(f)
    }

    #[inline]
    pub fn with_mut<F: FnOnce(&mut T) -> R, R>(&mut self, f: F) -> R {
        self.state.with_mut(f)
    }

    #[inline]
    fn ud_ptr(&self) -> *mut *mut RefCell<T> {
        self.state.ud_ptr()
    }
}

impl<T> StackUd<T> where T: UserData {}

impl<T> UdRef<T>
where
    T: UserData,
{
    pub fn try_clone(&self) -> Result<Self, Error> {
        let lua = self.state.lua.clone();
        let ud_ptr = self.state.ud_ptr;
        let id = unsafe {
            let ptr = lua.borrow().try_state()?;
            sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _);
            sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX)
        };

        Ok(Self {
            state: OwnedState { lua, id, ud_ptr },
        })
    }
}

impl<T> Clone for UdRef<T>
where
    T: UserData,
{
    fn clone(&self) -> Self {
        self.try_clone().unwrap_display()
    }
}

unsafe impl<T> FromLua for StackUd<T>
where
    T: UserData,
{
    fn try_from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Result<Self, Error> {
        unsafe {
            helper::try_check_stack(ptr, 2)?;
            let idx = sys::lua_absindex(ptr, idx);

            if sys::lua_getmetatable(ptr, idx) == 0 {
                return Err(Error::UnexpectedType);
            }

            sys::lua_rawgeti(ptr, -1, 1);
            let type_id = sys::lua_tolightuserdata(ptr, -1);
            sys::lua_pop(ptr, 1);

            let expected_type_id = T::functions().as_ptr() as *mut std::ffi::c_void;
            if type_id != expected_type_id {
                sys::lua_pop(ptr, 1);
                return Err(Error::UnexpectedType);
            }

            sys::lua_pop(ptr, 1);

            let ud_ptr = sys::lua_touserdata(ptr, idx) as *mut *mut RefCell<T>;
            Ok(StackUd::<T> {
                state: BorrowedState { ptr, idx, ud_ptr },
            })
        }
    }
}

unsafe impl<T> FromLua for UdRef<T>
where
    T: UserData,
{
    fn try_from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Result<Self, Error> {
        unsafe {
            helper::try_check_stack(ptr, 2)?;
            let idx = sys::lua_absindex(ptr, idx);

            if sys::lua_getmetatable(ptr, idx) == 0 {
                return Err(Error::UnexpectedType);
            }

            sys::lua_rawgeti(ptr, -1, 1);
            let type_id = sys::lua_tolightuserdata(ptr, -1);
            sys::lua_pop(ptr, 1);

            let expected_type_id = T::functions().as_ptr() as *mut std::ffi::c_void;
            if type_id != expected_type_id {
                sys::lua_pop(ptr, 1);
                return Err(Error::UnexpectedType);
            }

            sys::lua_pop(ptr, 1);

            sys::lua_pushvalue(ptr, idx);
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);

            let lua = RefCell::new(InnerLua::from_ptr(ptr));
            let ud_ptr = sys::lua_touserdata(ptr, idx) as *mut *mut RefCell<T>;
            let ud = UdRef::<T> {
                state: OwnedState { lua, id, ud_ptr },
            };

            Ok(ud)
        }
    }
}

unsafe impl<T> ToLua for StackUd<T>
where
    T: UserData,
{
    unsafe fn try_to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) -> Result<(), Error> {
        InnerLua::try_ensure_context_raw(self.state.ptr, ptr)?;
        unsafe { sys::lua_pushvalue(ptr, self.state.idx) }
        Ok(())
    }
}

unsafe impl<T> ToLua for &UdRef<T>
where
    T: UserData,
{
    unsafe fn try_to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) -> Result<(), Error> {
        InnerLua::try_ensure_context_raw(self.state.lua.borrow().try_state()?, ptr)?;
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _) };
        Ok(())
    }
}

unsafe impl<T> ToLua for UdRef<T>
where
    T: UserData,
{
    unsafe fn try_to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) -> Result<(), Error> {
        unsafe { (&self).try_to_lua_unchecked(ptr) }
    }
}

impl<M, T> IsType for Ud<M, T>
where
    M: Mode + UserDataState<T>,
    M::State: UserDataAccess<T>,
    T: UserData,
{
    fn is_type(ptr: *mut mlua_sys::lua_State, idx: i32) -> bool {
        unsafe { sys::lua_type(ptr, idx) == sys::LUA_TUSERDATA }
    }
}

impl<M1, M2, T> PartialEq<Ud<M2, T>> for Ud<M1, T>
where
    M1: Mode + UserDataState<T>,
    M1::State: UserDataAccess<T>,
    M2: Mode + UserDataState<T>,
    M2::State: UserDataAccess<T>,
    T: UserData,
{
    fn eq(&self, other: &Ud<M2, T>) -> bool {
        self.ud_ptr() == other.ud_ptr()
    }
}

impl<M, T> Eq for Ud<M, T>
where
    M: Mode + UserDataState<T>,
    M::State: UserDataAccess<T>,
    T: UserData,
{
}

impl<M, T> Hash for Ud<M, T>
where
    M: Mode + UserDataState<T>,
    M::State: UserDataAccess<T>,
    T: UserData,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ud_ptr().hash(state);
    }
}

impl<T> crate::owned_value::private::Sealed for UdRef<T> where T: UserData {}

impl<T> OwnedValue for UdRef<T>
where
    T: UserData,
{
    fn handle(&self) -> LuaInnerHandle<'_> {
        LuaInnerHandle(&self.state.lua)
    }
}
