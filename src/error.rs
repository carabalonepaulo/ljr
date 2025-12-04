use std::ffi::NulError;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("invalid string, interior nul byte found")]
    InvalidCString,
    #[error("lua error: unknown")]
    UnknownLuaError,
    #[error("lua error: {0}")]
    LuaError(String),
    #[error("unexpected type")]
    UnexpectedType,
    #[error("invalid syntax: {0}")]
    InvalidSyntax(String),
    #[error("wrong return type")]
    WrongReturnType,
    #[error("missing global: {0}")]
    MissingGlobal(String),
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Error::InvalidCString
    }
}

impl Error {
    pub(crate) unsafe fn from_stack(ptr: *mut crate::sys::lua_State, idx: i32) -> Error {
        if let Some(msg) = <String as crate::from_lua::FromLua>::from_lua(ptr, idx) {
            unsafe { crate::sys::lua_pop(ptr, 1) };
            return Error::LuaError(msg);
        } else {
            unsafe { crate::sys::lua_pop(ptr, 1) };
            return Error::UnknownLuaError;
        }
    }
}
