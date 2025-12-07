use std::{
    cell::{BorrowError, BorrowMutError},
    ffi::NulError,
    fmt::Display,
    str::Utf8Error,
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
    #[error(transparent)]
    Utf8Error(#[from] Utf8Error),
    #[error("wrong number of arguments, expecting {0}, got {1}")]
    ArgumentCountMismatch(usize, usize),
    #[error("invalid argument {0}, expected {0}")]
    ArgumentTypeMismatch(usize, String),
    #[error("insufficient values on stack: type requires {0}, but only {1} are available")]
    InsufficientStackValues(i32, i32),
    #[error("table is empty")]
    TableIsEmpty,
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Error::InvalidCString
    }
}

impl Error {
    pub(crate) unsafe fn from_stack(ptr: *mut crate::sys::lua_State, idx: i32) -> Error {
        if let Ok(msg) = <String as crate::from_lua::FromLua>::try_from_lua(ptr, idx) {
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
