use crate::s3_path::ParseS3PathError;
use crate::utils::BoxStdError;

use std::error::Error;
use std::fmt::{self, Display};

// TODO: switch to thiserror
// See https://github.com/dtolnay/thiserror/issues/79

pub type S3Result<T, E = NopError> = Result<T, S3Error<E>>;

#[derive(Debug)]
pub enum S3Error<E = NopError> {
    Operation(E),
    InvalidRequest(InvalidRequestError),
    InvalidOutput(InvalidOutputError),
    Storage(BoxStdError),
    NotSupported,
}

#[derive(Debug, Clone, Copy)]
pub struct NopError;

#[derive(Debug, thiserror::Error)]
pub enum InvalidRequestError {
    #[error(transparent)]
    ParsePath(#[from] ParseS3PathError),
    // FIXME: add other errors
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidOutputError {
    #[error(transparent)]
    InvalidHeaderName(#[from] hyper::header::InvalidHeaderName),

    #[error(transparent)]
    InvalidHeaderValue(#[from] hyper::header::InvalidHeaderValue),

    #[error(transparent)]
    XmlWriter(#[from] xml::writer::Error),
}

impl Display for NopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "()")
    }
}

impl Error for NopError {}

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
            Self::InvalidRequest(e) => Some(e),
            Self::InvalidOutput(e) => Some(e),
            Self::Storage(err) => Some(err.as_ref()),
            Self::NotSupported => None,
        }
    }
}
