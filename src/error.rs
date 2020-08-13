use std::error::Error;
use std::fmt::{self, Display};

type BoxStdError = Box<dyn Error + Send + Sync + 'static>;

// TODO: switch to thiserror, see https://github.com/dtolnay/thiserror/issues/79

#[derive(Debug)]
pub enum S3Error<E = NopError> {
    Operation(E),
    InvalidRequest(BoxStdError),
    InvalidOutput(BoxStdError),
    NotSupported,
}

#[derive(Debug, Clone, Copy)]
pub struct NopError;

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
            Self::NotSupported => write!(f, "Not supported"),
        }
    }
}

impl<E: Error + 'static> Error for S3Error<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Operation(e) => Some(e),
            Self::InvalidRequest(e) | Self::InvalidOutput(e) => Some(e.as_ref()),
            Self::NotSupported => None,
        }
    }
}
