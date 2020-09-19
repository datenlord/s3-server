//! Generic s3 error type.

use crate::BoxStdError;

use std::convert::Infallible as Never;
use std::error::Error;
use std::fmt::{self, Display};

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
    /// An error occurred when authenticating a request
    Auth(BoxStdError),
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
            Self::Auth(e) => write!(f, "Auth: {}", e),
            Self::NotSupported => write!(f, "Not supported"),
        }
    }
}

impl<E: Error + 'static> Error for S3Error<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Operation(e1) => Some(e1),
            Self::InvalidRequest(e2) => Some(e2.as_ref()),
            Self::InvalidOutput(e3) => Some(e3.as_ref()),
            Self::Storage(e4) => Some(e4.as_ref()),
            Self::Auth(e5) => Some(e5.as_ref()),
            Self::NotSupported => None,
        }
    }
}
