//! Collection of own error-related types

use neovim_lib;
use std::{fmt, io};

/// Own error type
///
/// This is enum of all error types used by dependencies of this project
#[derive(Debug)]
pub enum Error {
    /// An [io::Error] variant
    Io(io::Error),
    /// A [neovim_lib::CallError] variant
    Neovim(neovim_lib::CallError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => e.fmt(f),
            Error::Neovim(ref e) => e.fmt(f),
        }
    }
}

#[allow(deprecated)]
impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
            Error::Neovim(ref e) => e.description(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<Error> for io::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::Io(io) => io,
            _ => io::Error::new(io::ErrorKind::InvalidData, value),
        }
    }
}

impl From<neovim_lib::CallError> for Error {
    fn from(value: neovim_lib::CallError) -> Self {
        Self::Neovim(value)
    }
}

/// Own result type
///
/// This is result based on [Error]
pub type Result<T> = std::result::Result<T, Error>;
