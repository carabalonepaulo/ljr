pub mod error;
pub mod from_lua;
pub mod helper;
mod is_type;
pub mod lua;
pub mod lua_ref;
pub mod stack_ref;
// pub mod borrow_from_lua;
// pub mod lua_string;
pub mod stack;
pub mod to_lua;

pub struct AnyUserData;

pub struct AnyLuaFunction;

pub struct AnyNativeFunction;

pub struct Coroutine;

pub struct LightUserData;

pub struct Table;

pub struct Nil;

pub trait UserData {
    fn name() -> *const i8;
    fn functions() -> Vec<luajit2_sys::luaL_Reg>;
}
