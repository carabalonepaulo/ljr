pub mod defer;
pub mod error;
pub mod helper;

pub mod lua;
pub mod stack;

pub mod lua_ref;
pub mod stack_ref;
pub mod stack_str;
pub mod table;

pub mod from_lua;
pub mod is_type;
pub mod to_lua;

pub use luajit2_sys as sys;
pub use macros::*;

pub struct AnyUserData;

pub struct AnyLuaFunction;

pub struct AnyNativeFunction;

pub struct Coroutine;

pub struct LightUserData;

pub struct Nil;

pub trait UserData {
    fn name() -> *const i8;
    fn functions() -> Vec<luajit2_sys::luaL_Reg>;
}

pub mod prelude {
    pub use crate::UserData;
    pub use crate::error::Error;
    pub use crate::lua::Lua;
    pub use crate::lua_ref::LuaRef;
    pub use crate::table::Table;
    pub use macros::user_data;
}
