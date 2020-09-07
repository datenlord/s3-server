//! Generic s3 error type.

use crate::BoxStdError;

use std::convert::Infallible as Never;
use std::error::Error;
use std::fmt::{self, Display};

// TODO: switch to thiserror
// See https://github.com/dtolnay/thiserror/issues/79

/// Result carrying a generic `S3Error<E>`
pub type S3Result<T, E = Never> = Result<T, S3Error<E>>;

/// Generic s3 error type.
#[derive(Debug)]
pub enum S3Error<E = Never> {
    /// A operation-specific error occurred
    Operation(E),
    /// An error occurred when parsing and validating a request
    InvalidRequest(BoxStdError),
    /// An error occurred when converting storage output to a response
    InvalidOutput(BoxStdError),
    /// An error occurred when operating the storage
    Storage(BoxStdError),
    /// An error occurred when the operation is not supported
    NotSupported,
}

impl<E: Display> Display for S3Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operation(e) => write!(f, "Operation: {}", e),
            Self::InvalidRequest(e) => write!(f, "Invalid request: {}", e),
            Self::InvalidOutput(e) => write!(f, "Invalid output: {}", e),
            Self::Storage(e) => write!(f, "Storage: {}", e),
            Self::NotSupported => write!(f, "Not supported"),
        }
    }
}

impl<E: Error + 'static> Error for S3Error<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Operation(e) => Some(e),
            Self::InvalidRequest(e) => Some(e.as_ref()),
            Self::InvalidOutput(er) => Some(er.as_ref()),
            Self::Storage(err) => Some(err.as_ref()),
            Self::NotSupported => None,
        }
    }
}
