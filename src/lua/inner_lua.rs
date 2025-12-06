use crate::sys;
use std::{cell::Cell, ptr, rc::Rc};

use crate::error::Error;

static CTX_KEY: u8 = 0;

unsafe extern "C-unwind" fn individual_sentinel_gc(ptr: *mut sys::lua_State) -> i32 {
    unsafe {
        let weak_ptr = sys::lua_touserdata(ptr, 1) as *mut std::rc::Weak<InnerLua>;
        if !weak_ptr.is_null() {
            let weak = &*weak_ptr;
            if let Some(inner) = weak.upgrade() {
                inner.state.set(ptr::null_mut());
            }
            ptr::drop_in_place(weak_ptr);
        }
        0
    }
}

#[derive(Debug)]
pub struct InnerLua {
    state: Cell<*mut sys::lua_State>,
    owned: bool,
    thread_ref: Option<i32>,
    cache_key: *mut std::ffi::c_void,
    vm_id: *const std::ffi::c_void,
}

unsafe fn get_vm_id(ptr: *mut sys::lua_State) -> *const std::ffi::c_void {
    unsafe { sys::lua_topointer(ptr, sys::LUA_REGISTRYINDEX) }
}

impl InnerLua {
    pub(crate) fn new(ptr: *mut sys::lua_State) -> Rc<Self> {
        let cache_key = &CTX_KEY as *const u8 as *mut std::ffi::c_void;
        let inner = Rc::new(InnerLua {
            state: Cell::new(ptr),
            owned: true,
            thread_ref: None,
            cache_key,
            vm_id: unsafe { get_vm_id(ptr) },
        });
        unsafe { InnerLua::create_and_cache_sentinel(ptr, cache_key, inner.clone()) };
        inner
    }

    pub(crate) fn ensure_context_raw(a: *mut sys::lua_State, b: *mut sys::lua_State) {
        if unsafe { get_vm_id(a) != get_vm_id(b) } {
            panic!("cannot interact with values from a different lua state")
        }
    }

    pub(crate) fn assert_context(&self, other: &InnerLua) -> Result<(), Error> {
        if self.vm_id == other.vm_id {
            Ok(())
        } else {
            Err(Error::ContextMismatch)
        }
    }

    unsafe fn create_and_cache_sentinel(
        ptr: *mut sys::lua_State,
        key: *mut std::ffi::c_void,
        inner: Rc<InnerLua>,
    ) {
        unsafe {
            sys::lua_pushlightuserdata(ptr, key);
            let size = std::mem::size_of::<std::rc::Weak<InnerLua>>();
            let udata = sys::lua_newuserdata(ptr, size) as *mut std::rc::Weak<InnerLua>;

            ptr::write(udata, Rc::downgrade(&inner));

            if sys::luaL_newmetatable(ptr, c"__LJR_GUARD".as_ptr()) == 1 {
                sys::lua_pushstring(ptr, c"__gc".as_ptr());
                sys::lua_pushcfunction(ptr, individual_sentinel_gc);
                sys::lua_settable(ptr, -3);
            }
            sys::lua_setmetatable(ptr, -2);

            sys::lua_settable(ptr, sys::LUA_REGISTRYINDEX);
        }
    }

    pub unsafe fn try_main_state(&self) -> Result<Rc<InnerLua>, Error> {
        unsafe {
            let ptr = self.state();
            let cache_key = &CTX_KEY as *const u8 as *mut std::ffi::c_void;
            sys::lua_pushlightuserdata(ptr, cache_key);
            sys::lua_gettable(ptr, sys::LUA_REGISTRYINDEX);

            let data_ptr = sys::lua_touserdata(ptr, -1);
            sys::lua_pop(ptr, 1);

            if data_ptr.is_null() {
                return Err(Error::MainStateNotAvailable);
            }

            let weak_ptr = data_ptr as *mut std::rc::Weak<InnerLua>;
            let weak = &*weak_ptr;

            match weak.upgrade() {
                Some(rc) => Ok(rc),
                None => Err(Error::MainStateNotAvailable),
            }
        }
    }

    pub(crate) fn from_ptr(ptr: *mut sys::lua_State) -> Rc<Self> {
        unsafe {
            let is_main = sys::lua_pushthread(ptr) == 1;
            let thread_val_on_stack = !is_main;

            let cache_key = if is_main {
                sys::lua_pop(ptr, 1);
                &CTX_KEY as *const u8 as *mut std::ffi::c_void
            } else {
                ptr as *mut std::ffi::c_void
            };

            sys::lua_pushlightuserdata(ptr, cache_key);
            sys::lua_gettable(ptr, sys::LUA_REGISTRYINDEX);

            let data_ptr = sys::lua_touserdata(ptr, -1);
            sys::lua_pop(ptr, 1);

            if !data_ptr.is_null() {
                let weak_ptr = data_ptr as *mut std::rc::Weak<InnerLua>;
                let weak = &*weak_ptr;

                if let Some(rc) = weak.upgrade() {
                    if thread_val_on_stack {
                        sys::lua_pop(ptr, 1);
                    }
                    return rc;
                }
            }

            let thread_ref = if is_main {
                None
            } else {
                Some(sys::luaL_ref(ptr, sys::LUA_REGISTRYINDEX))
            };

            let inner = Rc::new(InnerLua {
                state: Cell::new(ptr),
                owned: false,
                thread_ref,
                cache_key,
                vm_id: get_vm_id(ptr),
            });

            Self::create_and_cache_sentinel(ptr, cache_key, inner.clone());
            inner
        }
    }

    pub(crate) fn state(&self) -> *mut sys::lua_State {
        let ptr = self.state.get();
        if ptr.is_null() {
            panic!("lua state has been closed");
        }
        ptr
    }

    pub(crate) fn try_state(&self) -> Result<*mut sys::lua_State, Error> {
        let ptr = self.state.get();
        if ptr.is_null() {
            Err(Error::LuaStateClosed)
        } else {
            Ok(ptr)
        }
    }
}

impl Drop for InnerLua {
    fn drop(&mut self) {
        let ptr = self.state.get();
        if !ptr.is_null() {
            unsafe {
                if let Some(r) = self.thread_ref {
                    sys::luaL_unref(ptr, sys::LUA_REGISTRYINDEX, r);
                }

                if self.owned {
                    sys::lua_close(ptr);
                } else {
                    sys::lua_pushlightuserdata(ptr, self.cache_key);
                    sys::lua_pushnil(ptr);
                    sys::lua_settable(ptr, sys::LUA_REGISTRYINDEX);
                }
            }
        }
    }
}

impl PartialEq for InnerLua {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key == other.cache_key
    }
}
