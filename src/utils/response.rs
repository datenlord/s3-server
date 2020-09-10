//! response util

use crate::Response;
use hyper::{
    header::{self, HeaderName, HeaderValue, InvalidHeaderValue},
    Body, StatusCode,
};
use mime::Mime;
use std::convert::TryFrom;

/// `ResponseExt`
pub trait ResponseExt {
    /// create response with body and status
    fn new_with_status(body: impl Into<Body>, status: StatusCode) -> Self;

    /// set optional header
    fn set_opt_header(
        &mut self,
        name: impl FnOnce() -> HeaderName,
        value: Option<String>,
    ) -> Result<(), InvalidHeaderValue>;

    /// set `Content-Type` by mime
    fn set_mime(&mut self, mime: &Mime) -> Result<(), InvalidHeaderValue>;

    /// set status code
    fn set_status(&mut self, status: StatusCode);
}

impl ResponseExt for Response {
    fn new_with_status(body: impl Into<Body>, status: StatusCode) -> Self {
        let mut res = Self::new(body.into());
        *res.status_mut() = status;
        res
    }

    fn set_opt_header(
        &mut self,
        name: impl FnOnce() -> HeaderName,
        value: Option<String>,
    ) -> Result<(), InvalidHeaderValue> {
        if let Some(value) = value {
            let val = HeaderValue::try_from(value)?;
            let _ = self.headers_mut().insert(name(), val);
        }
        Ok(())
    }

    fn set_mime(&mut self, mime: &Mime) -> Result<(), InvalidHeaderValue> {
        let val = HeaderValue::try_from(mime.as_ref())?;
        let _ = self.headers_mut().insert(header::CONTENT_TYPE, val);
        Ok(())
    }

    fn set_status(&mut self, status: StatusCode) {
        *self.status_mut() = status;
    }
}
