use crate::sys;
use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
};

use crate::{UserData, from_lua::FromLua};

#[derive(Debug)]
pub struct StackRef<T: FromLua + UserData> {
    ptr: *mut sys::lua_State,
    idx: i32,
    marker: PhantomData<T>,
}

impl<T: FromLua + UserData> StackRef<T> {
    pub fn new(ptr: *mut sys::lua_State, idx: i32) -> Self {
        Self {
            ptr,
            idx,
            marker: PhantomData,
        }
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        unsafe {
            let ud_ptr = crate::sys::lua_touserdata(self.ptr, self.idx) as *const *const RefCell<T>;
            let cell: &RefCell<T> = &**ud_ptr;
            cell.borrow()
        }
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        unsafe {
            let ud_ptr = crate::sys::lua_touserdata(self.ptr, self.idx) as *const *const RefCell<T>;
            let cell: &RefCell<T> = &**ud_ptr;
            cell.borrow_mut()
        }
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.borrow();
        f(&*guard)
    }

    pub fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.borrow_mut();
        f(&mut *guard)
    }
}
