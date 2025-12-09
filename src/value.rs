use std::{cell::RefCell, rc::Rc};

use crate::{
    Borrowed, Mode, Nil, Owned, UserData,
    error::{Error, UnwrapDisplay},
    from_lua::FromLua,
    func::{FnRef, StackFn},
    helper,
    lstr::{StackStr, StrRef},
    lua::InnerLua,
    owned_value::LuaInnerHandle,
    prelude::OwnedValue,
    stack_guard::StackGuard,
    sys,
    table::{StackTable, TableRef},
    to_lua::ToLua,
    ud::{StackUd, UdRef},
};

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Nil,
    Bool,
    Number,
    String,
    UserData,
    Func,
    // Thread,
    Table,
    Unknown,
}

impl Kind {
    fn try_from_stack(ptr: *mut sys::lua_State, idx: i32) -> Result<Kind, Error> {
        unsafe {
            match sys::lua_type(ptr, idx) {
                sys::LUA_TNIL => Ok(Kind::Nil),
                sys::LUA_TBOOLEAN => Ok(Kind::Bool),
                sys::LUA_TNUMBER => Ok(Kind::Number),
                sys::LUA_TSTRING => Ok(Kind::String),
                sys::LUA_TUSERDATA => Ok(Kind::UserData),
                sys::LUA_TFUNCTION => Ok(Kind::Func),
                // sys::LUA_TTHREAD => {},
                sys::LUA_TTABLE => Ok(Kind::Table),
                _ => Ok(Kind::Unknown),
            }
        }
    }
}

pub trait ValueState {
    type State;
}

pub trait ValueAccess {
    fn kind(&self) -> Kind;

    fn try_with_nil<F: FnOnce(Nil) -> R, R>(&self, f: F) -> Result<R, Error>;

    #[inline(always)]
    fn try_as_nil(&self) -> Result<Nil, Error> {
        self.try_with_nil(|v| v)
    }

    #[inline(always)]
    fn as_nil(&self) -> Nil {
        self.try_as_nil().unwrap_display()
    }

    fn try_with_bool<F: FnOnce(bool) -> R, R>(&self, f: F) -> Result<R, Error>;

    #[inline(always)]
    fn try_as_bool(&self) -> Result<bool, Error> {
        self.try_with_bool(|v| v)
    }

    #[inline(always)]
    fn as_bool(&self) -> bool {
        self.try_as_bool().unwrap_display()
    }

    fn try_with_number<F: FnOnce(f64) -> R, R>(&self, f: F) -> Result<R, Error>;

    #[inline(always)]
    fn try_as_number(&self) -> Result<f64, Error> {
        self.try_with_number(|v| v)
    }

    #[inline(always)]
    fn as_number(&self) -> f64 {
        self.try_as_number().unwrap_display()
    }

    fn try_with_str<F: FnOnce(&StackStr) -> R, R>(&self, f: F) -> Result<R, Error>;

    #[inline(always)]
    fn try_as_str(&self) -> Result<StrRef, Error> {
        self.try_with_str(|v| v.try_to_owned()).flatten()
    }

    #[inline(always)]
    fn as_str(&self) -> StrRef {
        self.try_as_str().unwrap_display()
    }

    fn try_with_ud<T: UserData, F: FnOnce(&StackUd<T>) -> R, R>(&self, f: F) -> Result<R, Error>;

    #[inline(always)]
    fn try_as_ud<T: UserData>(&self) -> Result<UdRef<T>, Error> {
        self.try_with_ud(|v| v.try_to_owned()).flatten()
    }

    #[inline(always)]
    fn as_ud<T: UserData>(&self) -> UdRef<T> {
        self.try_as_ud().unwrap_display()
    }

    fn try_with_func<I, O, F, R>(&self, f: F) -> Result<R, Error>
    where
        I: FromLua + ToLua,
        O: FromLua + ToLua,
        F: FnOnce(&StackFn<I, O>) -> R;

    #[inline(always)]
    fn try_as_func<I, O>(&self) -> Result<FnRef<I, O>, Error>
    where
        I: FromLua + ToLua,
        O: FromLua + ToLua,
    {
        self.try_with_func(|v| v.try_to_owned()).flatten()
    }

    #[inline(always)]
    fn as_func<I, O>(&self) -> FnRef<I, O>
    where
        I: FromLua + ToLua,
        O: FromLua + ToLua,
    {
        self.try_as_func().unwrap_display()
    }

    fn try_with_table<F: FnOnce(&StackTable) -> R, R>(&self, f: F) -> Result<R, Error>;

    #[inline(always)]
    fn try_as_table(&self) -> Result<TableRef, Error> {
        self.try_with_table(|v| v.try_to_owned()).flatten()
    }

    #[inline(always)]
    fn as_table(&self) -> TableRef {
        self.try_as_table().unwrap_display()
    }
}

pub struct BorrowedState {
    ptr: *mut sys::lua_State,
    idx: i32,
    kind: Kind,
}

impl ValueState for Borrowed {
    type State = BorrowedState;
}

impl ValueAccess for BorrowedState {
    fn kind(&self) -> Kind {
        self.kind
    }

    fn try_with_nil<F: FnOnce(Nil) -> R, R>(&self, f: F) -> Result<R, Error> {
        match self.kind {
            Kind::Nil => Ok(f(Nil)),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_bool<F: FnOnce(bool) -> R, R>(&self, f: F) -> Result<R, Error> {
        match self.kind {
            Kind::Bool => Ok(f(bool::try_from_lua(self.ptr, self.idx)?)),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_number<F: FnOnce(f64) -> R, R>(&self, f: F) -> Result<R, Error> {
        match self.kind {
            Kind::Number => Ok(f(f64::try_from_lua(self.ptr, self.idx)?)),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_str<F: FnOnce(&StackStr) -> R, R>(&self, f: F) -> Result<R, Error> {
        match self.kind {
            Kind::String => Ok(f(&StackStr::try_from_lua(self.ptr, self.idx)?)),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_ud<T: UserData, F: FnOnce(&StackUd<T>) -> R, R>(&self, f: F) -> Result<R, Error> {
        match self.kind {
            Kind::UserData => Ok(f(&StackUd::<T>::try_from_lua(self.ptr, self.idx)?)),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_func<I, O, F, R>(&self, f: F) -> Result<R, Error>
    where
        I: FromLua + ToLua,
        O: FromLua + ToLua,
        F: FnOnce(&StackFn<I, O>) -> R,
    {
        match self.kind {
            Kind::Func => Ok(f(&StackFn::<I, O>::try_from_lua(self.ptr, self.idx)?)),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_table<F: FnOnce(&StackTable) -> R, R>(&self, f: F) -> Result<R, Error> {
        match self.kind {
            Kind::Table => Ok(f(&StackTable::try_from_lua(self.ptr, self.idx)?)),
            _ => Err(Error::UnexpectedType),
        }
    }
}

#[allow(unused)]
pub struct OwnedState {
    lua: RefCell<Rc<InnerLua>>,
    id: i32,
    kind: Kind,
}

impl OwnedState {
    fn with_value<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(*mut sys::lua_State) -> Result<R, Error>,
    {
        unsafe {
            let ptr = self.lua.borrow().try_state()?;
            let _g = StackGuard::new(ptr);
            sys::lua_rawgeti_(ptr, sys::LUA_REGISTRYINDEX, self.id);
            let result = f(ptr);
            result
        }
    }
}

impl Drop for OwnedState {
    fn drop(&mut self) {
        if let Ok(ptr) = self.lua.borrow().try_state() {
            unsafe { sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, self.id) };
        }
    }
}

impl ValueState for Owned {
    type State = OwnedState;
}

impl ValueAccess for OwnedState {
    fn kind(&self) -> Kind {
        self.kind
    }

    fn try_with_nil<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(Nil) -> R,
    {
        match self.kind {
            Kind::Nil => Ok(f(Nil)),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_bool<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(bool) -> R,
    {
        match self.kind {
            Kind::Bool => self.with_value(|ptr| Ok(f(bool::try_from_lua(ptr, -1)?))),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_number<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(f64) -> R,
    {
        match self.kind {
            Kind::Number => self.with_value(|ptr| Ok(f(f64::try_from_lua(ptr, -1)?))),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_str<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&StackStr) -> R,
    {
        match self.kind {
            Kind::String => self.with_value(|ptr| Ok(f(&StackStr::try_from_lua(ptr, -1)?))),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_ud<T, F, R>(&self, f: F) -> Result<R, Error>
    where
        T: UserData,
        F: FnOnce(&StackUd<T>) -> R,
    {
        match self.kind {
            Kind::UserData => self.with_value(|ptr| Ok(f(&StackUd::<T>::try_from_lua(ptr, -1)?))),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_func<I, O, F, R>(&self, f: F) -> Result<R, Error>
    where
        I: FromLua + ToLua,
        O: FromLua + ToLua,
        F: FnOnce(&StackFn<I, O>) -> R,
    {
        match self.kind {
            Kind::Func => self.with_value(|ptr| Ok(f(&StackFn::<I, O>::try_from_lua(ptr, -1)?))),
            _ => Err(Error::UnexpectedType),
        }
    }

    fn try_with_table<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&StackTable) -> R,
    {
        match self.kind {
            Kind::Table => self.with_value(|ptr| Ok(f(&StackTable::try_from_lua(ptr, -1)?))),
            _ => Err(Error::UnexpectedType),
        }
    }
}

pub type StackValue = Value<Borrowed>;
pub type ValueRef = Value<Owned>;

pub struct Value<M>
where
    M: Mode + ValueState,
    M::State: ValueAccess,
{
    state: M::State,
}

impl<M> Value<M>
where
    M: Mode + ValueState,
    M::State: ValueAccess,
{
    #[inline(always)]
    pub fn kind(&self) -> Kind {
        self.state.kind()
    }

    #[inline(always)]
    pub fn try_with_nil<F: FnOnce(Nil) -> R, R>(&self, f: F) -> Result<R, Error> {
        self.state.try_with_nil(f)
    }

    #[inline(always)]
    pub fn try_as_nil(&self) -> Result<Nil, Error> {
        self.state.try_as_nil()
    }

    #[inline(always)]
    pub fn as_nil(&self) -> Nil {
        self.state.as_nil()
    }

    #[inline(always)]
    pub fn try_with_bool<F: FnOnce(bool) -> R, R>(&self, f: F) -> Result<R, Error> {
        self.state.try_with_bool(f)
    }

    #[inline(always)]
    pub fn try_as_bool(&self) -> Result<bool, Error> {
        self.state.try_as_bool()
    }

    #[inline(always)]
    pub fn as_bool(&self) -> bool {
        self.state.as_bool()
    }

    #[inline(always)]
    pub fn try_with_number<F: FnOnce(f64) -> R, R>(&self, f: F) -> Result<R, Error> {
        self.state.try_with_number(f)
    }

    #[inline(always)]
    pub fn try_as_number(&self) -> Result<f64, Error> {
        self.state.try_as_number()
    }

    #[inline(always)]
    pub fn as_number(&self) -> f64 {
        self.state.as_number()
    }

    #[inline(always)]
    pub fn try_with_str<F: FnOnce(&StackStr) -> R, R>(&self, f: F) -> Result<R, Error> {
        self.state.try_with_str(f)
    }

    #[inline(always)]
    pub fn try_as_str(&self) -> Result<StrRef, Error> {
        self.state.try_as_str()
    }

    #[inline(always)]
    pub fn as_str(&self) -> StrRef {
        self.state.as_str()
    }

    #[inline(always)]
    pub fn try_with_ud<T: UserData, F: FnOnce(&StackUd<T>) -> R, R>(
        &self,
        f: F,
    ) -> Result<R, Error> {
        self.state.try_with_ud(f)
    }

    #[inline(always)]
    pub fn try_as_ud<T: UserData>(&self) -> Result<UdRef<T>, Error> {
        self.state.try_as_ud()
    }

    #[inline(always)]
    pub fn as_ud<T: UserData>(&self) -> UdRef<T> {
        self.state.as_ud()
    }

    #[inline(always)]
    pub fn try_with_func<I, O, F, R>(&self, f: F) -> Result<R, Error>
    where
        I: FromLua + ToLua,
        O: FromLua + ToLua,
        F: FnOnce(&StackFn<I, O>) -> R,
    {
        self.state.try_with_func(f)
    }

    #[inline(always)]
    pub fn try_as_func<I, O>(&self) -> Result<FnRef<I, O>, Error>
    where
        I: FromLua + ToLua,
        O: FromLua + ToLua,
    {
        self.state.try_as_func()
    }

    #[inline(always)]
    pub fn as_func<I, O>(&self) -> FnRef<I, O>
    where
        I: FromLua + ToLua,
        O: FromLua + ToLua,
    {
        self.state.as_func()
    }

    #[inline(always)]
    pub fn try_with_table<F: FnOnce(&StackTable) -> R, R>(&self, f: F) -> Result<R, Error> {
        self.state.try_with_table(f)
    }

    #[inline(always)]
    pub fn try_as_table(&self) -> Result<TableRef, Error> {
        self.state.try_as_table()
    }

    #[inline(always)]
    pub fn as_table(&self) -> TableRef {
        self.state.as_table()
    }
}

impl StackValue {
    #[inline(always)]
    pub fn try_to_owned(&self) -> Result<ValueRef, Error> {
        ValueRef::try_from_stack(self.state.ptr, self.state.idx)
    }

    #[inline(always)]
    pub fn to_owned(&self) -> ValueRef {
        ValueRef::try_from_stack(self.state.ptr, self.state.idx).unwrap_display()
    }
}

impl ValueRef {
    pub(crate) fn try_from_stack(ptr: *mut sys::lua_State, idx: i32) -> Result<ValueRef, Error> {
        unsafe {
            let lua = RefCell::new(InnerLua::from_ptr(ptr));
            sys::lua_pushvalue(ptr, idx);
            let kind = Kind::try_from_stack(ptr, idx)?;
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);

            Ok(Self {
                state: OwnedState { lua, id, kind },
            })
        }
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        let lua = self.state.lua.clone();
        let ptr = lua.borrow().try_state()?;
        let id = unsafe {
            sys::lua_rawgeti_(ptr, sys::LUA_REGISTRYINDEX, self.state.id);
            sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX)
        };
        let kind = self.state.kind;

        let state = OwnedState { id, lua, kind };
        Ok(Self { state })
    }
}

impl Clone for ValueRef {
    #[inline]
    fn clone(&self) -> Self {
        self.try_clone().unwrap_display()
    }
}

unsafe impl FromLua for StackValue {
    fn try_from_lua(ptr: *mut sys::lua_State, idx: i32) -> Result<Self, Error> {
        unsafe {
            let idx = sys::lua_absindex(ptr, idx);
            let kind = Kind::try_from_stack(ptr, idx)?;
            Ok(StackValue {
                state: BorrowedState { ptr, idx, kind },
            })
        }
    }
}

unsafe impl FromLua for ValueRef {
    fn try_from_lua(ptr: *mut mlua_sys::lua_State, idx: i32) -> Result<Self, Error> {
        unsafe {
            helper::try_check_stack(ptr, 1)?;
            let lua = RefCell::new(InnerLua::from_ptr(ptr));
            sys::lua_pushvalue(ptr, idx);
            let kind = Kind::try_from_stack(ptr, idx)?;
            let id = sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX);

            Ok(Self {
                state: OwnedState { lua, id, kind },
            })
        }
    }
}

unsafe impl ToLua for &StackValue {
    unsafe fn try_to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) -> Result<(), Error> {
        InnerLua::try_ensure_context_raw(self.state.ptr, ptr)?;
        Ok(unsafe { sys::lua_pushvalue(ptr, self.state.idx) })
    }
}

unsafe impl ToLua for StackValue {
    unsafe fn try_to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) -> Result<(), Error> {
        unsafe { (&self).try_to_lua_unchecked(ptr) }
    }
}

unsafe impl ToLua for &ValueRef {
    unsafe fn try_to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) -> Result<(), Error> {
        InnerLua::try_ensure_context_raw(self.state.lua.borrow().try_state()?, ptr)?;
        unsafe { sys::lua_rawgeti(ptr, sys::LUA_REGISTRYINDEX, self.state.id as _) };
        Ok(())
    }
}

unsafe impl ToLua for ValueRef {
    unsafe fn try_to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) -> Result<(), Error> {
        unsafe { (&self).try_to_lua_unchecked(ptr) }
    }
}

impl<M> std::fmt::Debug for Value<M>
where
    M: Mode + ValueState,
    M::State: ValueAccess,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.state.kind() {
            Kind::Nil => write!(f, "Nil"),
            Kind::Bool => write!(f, "Bool({})", self.as_bool()),
            Kind::Number => write!(f, "Number({})", self.as_number()),
            Kind::String => write!(f, "String({:?})", self.as_str().as_str()),
            Kind::Table => write!(f, "Table"),
            Kind::Func => write!(f, "Function"),
            Kind::UserData => write!(f, "UserData"),
            Kind::Unknown => write!(f, "Unknown"),
        }
    }
}

impl crate::owned_value::private::Sealed for ValueRef {}

impl OwnedValue for ValueRef {
    fn handle(&self) -> LuaInnerHandle<'_> {
        LuaInnerHandle(&self.state.lua)
    }
}
