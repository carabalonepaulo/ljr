use std::cell::RefCell;

use crate::{
    error::{Error, UnwrapDisplay},
    sys,
};
use macros::generate_to_lua_tuple_impl;

use crate::UserData;

pub unsafe trait ToLua: Sized {
    const LEN: i32 = 1;

    unsafe fn to_lua_unchecked(self, ptr: *mut sys::lua_State);

    fn try_to_lua(self, ptr: *mut sys::lua_State) -> Result<(), Error> {
        unsafe {
            crate::helper::try_check_stack(ptr, Self::len())?;
            self.to_lua_unchecked(ptr);
        }
        Ok(())
    }

    fn to_lua(self, ptr: *mut sys::lua_State) {
        self.try_to_lua(ptr).unwrap_display()
    }

    fn len() -> i32 {
        Self::LEN
    }
}

unsafe impl ToLua for &i32 {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { sys::lua_pushinteger(ptr, (*self) as _) }
    }
}

unsafe impl ToLua for i32 {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { (&self).to_lua_unchecked(ptr) };
    }
}

unsafe impl ToLua for &f32 {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { sys::lua_pushnumber(ptr, (*self) as _) }
    }
}

unsafe impl ToLua for f32 {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { (&self).to_lua_unchecked(ptr) };
    }
}

unsafe impl ToLua for &f64 {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { sys::lua_pushnumber(ptr, *self) }
    }
}

unsafe impl ToLua for f64 {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { (&self).to_lua_unchecked(ptr) };
    }
}

unsafe impl ToLua for &bool {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { sys::lua_pushboolean(ptr, if *self { 1 } else { 0 }) }
    }
}

unsafe impl ToLua for bool {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { sys::lua_pushboolean(ptr, if self { 1 } else { 0 }) }
    }
}

unsafe impl ToLua for &str {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { sys::lua_pushlstring_(ptr, self.as_bytes().as_ptr() as *const i8, self.len()) }
    }
}

unsafe impl ToLua for &String {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { self.as_str().to_lua_unchecked(ptr) };
    }
}

unsafe impl ToLua for String {
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        unsafe { (&self).to_lua_unchecked(ptr) };
    }
}

unsafe impl<T> ToLua for T
where
    T: UserData,
{
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        let size = std::mem::size_of::<*mut RefCell<T>>();
        let name = T::name();
        let methods = T::functions();
        let self_ptr = Box::into_raw(Box::new(RefCell::new(self)));

        unsafe {
            let managed_ptr = sys::lua_newuserdata(ptr, size) as *mut *mut RefCell<T>;
            *managed_ptr = self_ptr;

            if sys::luaL_newmetatable(ptr, name) != 0 {
                let mt_idx = sys::lua_gettop(ptr);

                let type_id = T::functions().as_ptr() as *mut std::ffi::c_void;
                sys::lua_pushlightuserdata(ptr, type_id);
                sys::lua_rawseti(ptr, mt_idx, 1);

                sys::lua_pushstring(ptr, name);
                sys::lua_setfield(ptr, mt_idx, c"__name".as_ptr());

                unsafe extern "C-unwind" fn __gc<T: UserData>(
                    ptr: *mut crate::sys::lua_State,
                ) -> i32 {
                    unsafe {
                        let ud_ptr = sys::lua_touserdata(ptr, 1) as *mut *mut RefCell<T>;
                        if !ud_ptr.is_null() && !(*ud_ptr).is_null() {
                            std::mem::drop(Box::from_raw(*ud_ptr));
                            *ud_ptr = std::ptr::null_mut();
                        }
                    }
                    0
                }
                sys::lua_pushcclosure(ptr, __gc::<T>, 0);
                sys::lua_setfield(ptr, mt_idx, c"__gc".as_ptr());

                sys::lua_newtable(ptr);
                sys::luaL_register(ptr, std::ptr::null(), methods.as_ptr());
                sys::lua_setfield(ptr, mt_idx, c"__index".as_ptr());
            }

            sys::lua_setmetatable(ptr, -2);
        }
    }
}

unsafe impl<T> ToLua for Option<T>
where
    T: ToLua,
{
    unsafe fn to_lua_unchecked(self, ptr: *mut crate::sys::lua_State) {
        match self {
            Some(value) => unsafe { value.to_lua_unchecked(ptr) },
            None => {
                for _ in 0..<T as ToLua>::len() {
                    unsafe { sys::lua_pushnil(ptr) };
                }
            }
        }
    }

    fn len() -> i32 {
        <T as ToLua>::len()
    }
}

unsafe impl ToLua for &[u8] {
    unsafe fn to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) {
        unsafe { sys::lua_pushlstring(ptr, self.as_ptr() as *const i8, self.len()) };
    }
}

unsafe impl ToLua for Vec<u8> {
    unsafe fn to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) {
        unsafe { (self.as_ref() as &[u8]).to_lua_unchecked(ptr) };
    }
}

unsafe impl ToLua for () {
    const LEN: i32 = 0;

    unsafe fn to_lua_unchecked(self, _: *mut mlua_sys::lua_State) {}
}

unsafe impl<T, E> ToLua for Result<T, E>
where
    T: ToLua,
    E: ToLua,
{
    unsafe fn to_lua_unchecked(self, ptr: *mut mlua_sys::lua_State) {
        match self {
            Ok(value) => {
                unsafe { value.to_lua_unchecked(ptr) };

                for _ in 0..E::len() {
                    unsafe { sys::lua_pushnil(ptr) };
                }
            }
            Err(e) => {
                for _ in 0..T::len() {
                    unsafe { sys::lua_pushnil(ptr) };
                }

                unsafe { e.to_lua_unchecked(ptr) };
            }
        }
    }

    fn len() -> i32 {
        T::len() + E::len()
    }
}

generate_to_lua_tuple_impl!();
