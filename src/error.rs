//! Generic s3 error type.

use crate::{BoxStdError, S3ErrorCode};

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
    /// Other errors
    Other(XmlErrorResponse),
    /// An error occurred when the operation is not supported
    NotSupported,
}

/// Type representing an error response
#[derive(Debug)]
pub struct XmlErrorResponse {
    /// code
    pub code: S3ErrorCode,
    /// message
    pub message: Option<String>,
    /// resource
    pub resource: Option<String>,
    /// request_id
    pub request_id: Option<String>,
}

impl XmlErrorResponse {
    /// Constructs a `XmlErrorResponse`
    pub(crate) const fn from_code_msg(code: S3ErrorCode, message: String) -> Self {
        Self {
            code,
            message: Some(message),
            resource: None,
            request_id: None,
        }
    }
}

impl<E: Display> Display for S3Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Operation(ref e) => write!(f, "Operation: {}", e),
            Self::InvalidRequest(ref e) => write!(f, "Invalid request: {}", e),
            Self::InvalidOutput(ref e) => write!(f, "Invalid output: {}", e),
            Self::Storage(ref e) => write!(f, "Storage: {}", e),
            Self::Auth(ref e) => write!(f, "Auth: {}", e),
            Self::Other(ref e) => write!(f, "Other: {:?}", e),
            Self::NotSupported => write!(f, "Not supported"),
        }
    }
}

impl<E: Error + 'static> Error for S3Error<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            Self::Operation(ref e1) => Some(e1),
            Self::InvalidRequest(ref e2) => Some(e2.as_ref()),
            Self::InvalidOutput(ref e3) => Some(e3.as_ref()),
            Self::Storage(ref e4) => Some(e4.as_ref()),
            Self::Auth(ref e5) => Some(e5.as_ref()),
            Self::Other(_) | Self::NotSupported => None,
        }
    }
}
