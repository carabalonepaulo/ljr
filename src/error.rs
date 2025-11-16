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
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Error::InvalidCString
    }
}
