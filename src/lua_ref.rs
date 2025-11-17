use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    rc::Rc,
};

use crate::UserData;

#[derive(Debug)]
struct Inner {
    ptr: *mut crate::sys::lua_State,
    id: i32,
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe { crate::sys::luaL_unref(self.ptr, crate::sys::LUA_REGISTRYINDEX, self.id) };
    }
}

#[derive(Debug)]
pub struct LuaRef<T: UserData> {
    inner: Rc<Inner>,
    marker: PhantomData<T>,
}

impl<T: UserData> LuaRef<T> {
    pub fn new(ptr: *mut crate::sys::lua_State) -> Self {
        let id = unsafe { crate::sys::luaL_ref(ptr, crate::sys::LUA_REGISTRYINDEX) };
        LuaRef {
            inner: Rc::new(Inner { ptr, id }),
            marker: PhantomData,
        }
    }

    pub fn id(&self) -> i32 {
        self.inner.id
    }

    pub fn as_ref(&self) -> Ref<'_, T> {
        let ptr = self.inner.ptr;
        let id = self.inner.id;

        unsafe {
            crate::sys::lua_rawgeti(ptr, crate::sys::LUA_REGISTRYINDEX, id as _);
            let ud_ptr = crate::sys::lua_touserdata(ptr, -1) as *const *const RefCell<T>;
            crate::sys::lua_pop(ptr, 1);

            let cell: &RefCell<T> = &**ud_ptr;
            cell.borrow()
        }
    }

    pub fn as_mut(&mut self) -> RefMut<'_, T> {
        let ptr = self.inner.ptr;
        let id = self.inner.id;

        unsafe {
            crate::sys::lua_rawgeti(ptr, crate::sys::LUA_REGISTRYINDEX, id as _);
            let ud_ptr = crate::sys::lua_touserdata(ptr, -1) as *const *const RefCell<T>;
            crate::sys::lua_pop(ptr, 1);

            let cell: &RefCell<T> = &**ud_ptr;
            cell.borrow_mut()
        }
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.as_ref();
        f(&*guard)
    }

    pub fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.as_mut();
        f(&mut *guard)
    }
}

impl<T: UserData> Clone for LuaRef<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: self.marker.clone(),
        }
    }
}
