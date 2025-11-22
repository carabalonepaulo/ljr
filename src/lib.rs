pub mod defer;
pub mod error;
pub mod helper;

pub mod lua;
pub mod stack;

pub mod fn_ref;
// pub mod lua_ref;
pub mod lua_str;
pub mod table;
pub mod ud;

pub mod stack_fn;
// pub mod stack_ref;
pub mod stack_str;

pub mod from_lua;
pub mod is_type;
pub mod to_lua;

pub use macros::*;
pub use mlua_sys as sys;

pub struct AnyUserData;

pub struct AnyLuaFunction;

pub struct AnyNativeFunction;

pub struct Coroutine;

pub struct LightUserData;

pub struct Nil;

pub trait UserData {
    fn name() -> *const i8;
    fn functions() -> &'static [crate::sys::luaL_Reg];
}

pub mod prelude {
    pub use crate::UserData;
    pub use crate::create_table;
    pub use crate::error::Error;
    pub use crate::fn_ref::FnRef;
    pub use crate::lua::Lua;
    pub use crate::ud::Ud;
    // pub use crate::lua_ref::LuaRef;
    pub use crate::lua_str::LuaStr;
    pub use crate::stack_fn::StackFn;
    pub use crate::table::Table;
    pub use macros::{module, user_data};
}

#[macro_export]
macro_rules! create_table {
    ($lua:expr, { $($item:tt)* }) => {{
        let mut table = $lua.create_table();
        table.with(|t| {
            create_table!(0, t, $($item)*);
        });
        table
    }};

    ($n:literal, $table:expr,) => {};

    ($n:literal, $table:expr, $value:expr, $($rest:tt)*) => {
        $table.push($value);
        create_table!(0, $table, $($rest)*);
    };

    ($n:literal, $table:expr, $value:expr) => { $table.push($value); };

    ($n:literal, $table:expr, $key:expr => $value:expr, $($rest:tt)*) => {
        $table.set($key, $value);
        create_table!(0, $table, $($rest)*);
    };

    ($n:literal, $table:expr, $key:expr => $value:expr) => { $table.set($key, $value); }
}

pub unsafe extern "C-unwind" fn dummy_trampoline(_: *mut crate::sys::lua_State) -> i32 {
    0
}
