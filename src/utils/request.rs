//! request util

use crate::Request;

use hyper::header::{AsHeaderName, HeaderValue, ToStrError};

/// `RequestExt`
pub trait RequestExt {
    /// get header value as `&str`
    fn get_header_str(&self, name: impl AsHeaderName) -> Result<Option<&str>, ToStrError>;
}

impl RequestExt for Request {
    fn get_header_str(&self, name: impl AsHeaderName) -> Result<Option<&str>, ToStrError> {
        self.headers()
            .get(name)
            .map(HeaderValue::to_str)
            .transpose()
    }
}
