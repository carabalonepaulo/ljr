pub mod error;
pub mod helper;

pub mod lua;

pub mod func;
pub mod lstr;
pub mod table;
pub mod ud;
pub mod value;

pub mod from_lua;
pub mod is_type;
pub mod to_lua;
pub use macros::*;
pub use mlua_sys as sys;

mod owned_value;
mod stack_guard;

pub struct AnyUserData;

pub struct AnyLuaFunction;

pub struct AnyNativeFunction;

pub struct Coroutine;

pub struct LightUserData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nil;

pub trait UserData {
    fn name() -> *const i8;
    fn functions() -> &'static [crate::sys::luaL_Reg];
}

pub mod prelude {
    pub use crate::Nil;
    pub use crate::UserData;
    pub use crate::create_table;
    pub use crate::error::{Error, UnwrapDisplay};
    pub use crate::func::{FnRef, StackFn};
    pub use crate::lstr::{StackStr, StrRef};
    pub use crate::lua::Lua;
    pub use crate::owned_value::OwnedValue;
    pub use crate::table::{StackTable, TableRef, builder::TableBuilder, view::TableView};
    pub use crate::ud::{StackUd, UdRef};
    pub use crate::value::{StackValue, ValueRef};
    pub use macros::{module, user_data};
}

pub trait Mode {}

pub struct Owned;
impl Mode for Owned {}

pub struct Borrowed;
impl Mode for Borrowed {}

#[repr(transparent)]
pub struct SyncLuaReg(pub sys::luaL_Reg);

unsafe impl Send for SyncLuaReg {}

unsafe impl Sync for SyncLuaReg {}

#[macro_export]
macro_rules! create_table {
    ($lua:expr, { $($item:tt)* }) => {{
        let mut table = $lua.create_table();
        table.with_mut(|t| {
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
