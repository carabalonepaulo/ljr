use crate::{from_lua::FromLua, lstr::StrRef, sys, to_lua::ToLua};

pub trait TableKey<'a> {
    type Output;

    fn to_lua(self, ptr: *mut sys::lua_State);

    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self::Output>;
}

pub trait TableValue<'a> {
    type Output;

    fn to_lua(self, ptr: *mut sys::lua_State);

    fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self::Output>;
}

macro_rules! impl_table_key {
    ($($ty:ty),*) => {
        $(
            impl<'a> TableKey<'a> for $ty {
                type Output = $ty;

                fn to_lua(self, ptr: *mut sys::lua_State) {
                    <Self as ToLua>::to_lua(self, ptr);
                }

                fn from_lua(ptr: *mut sys::lua_State, idx: i32) -> Option<Self::Output> {
                    <Self as FromLua>::from_lua(ptr, idx)
                }
            }
        )*
    };
}

impl_table_key!(i32, f32, f64, bool, String, StrRef);

impl<'a> TableKey<'a> for &str {
    type Output = String;

    fn to_lua(self, ptr: *mut crate::sys::lua_State) {
        <&str as ToLua>::to_lua(self, ptr);
    }

    fn from_lua(ptr: *mut crate::sys::lua_State, idx: i32) -> Option<Self::Output> {
        <String as FromLua>::from_lua(ptr, idx)
    }
}
