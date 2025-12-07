use std::{
    cell::{BorrowError, BorrowMutError},
    ffi::NulError,
    fmt::Display,
};

pub const STACK_OVERFLOW_ERR: &'static str = "cannot grow Lua stack to required size";

pub trait UnwrapDisplay<T> {
    fn unwrap_display(self) -> T;
}

impl<T, E: Display> UnwrapDisplay<T> for Result<T, E> {
    #[inline]
    #[track_caller]
    fn unwrap_display(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        }
    }
}

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
    #[error("cannot interact with values from a different lua state")]
    ContextMismatch,
    #[error("cannot modify value state: it is currently borrowed/in use")]
    ValueLocked,
    #[error(
        "main lua state is not available (library initialized inside a coroutine without explicit anchoring)"
    )]
    MainStateNotAvailable,
    #[error("cannot grow Lua stack to required size")]
    StackCapacityExceeded,
    #[error("lua state has been closed")]
    LuaStateClosed,
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Error::InvalidCString
    }
}

impl Error {
    pub(crate) unsafe fn from_stack(ptr: *mut crate::sys::lua_State, idx: i32) -> Error {
        if let Some(msg) = <String as crate::from_lua::FromLua>::from_lua(ptr, idx) {
            return Error::LuaError(msg);
        } else {
            return Error::UnknownLuaError;
        }
    }
}

impl From<BorrowError> for Error {
    fn from(_: BorrowError) -> Self {
        Self::ValueLocked
    }
}

impl From<BorrowMutError> for Error {
    fn from(_: BorrowMutError) -> Self {
        Self::ValueLocked
    }
}
