//! response util

use crate::Response;
use hyper::{
    header, header::HeaderName, header::HeaderValue, header::InvalidHeaderValue, Body, StatusCode,
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
        name: HeaderName,
        value: Option<String>,
    ) -> Result<(), InvalidHeaderValue>;

    /// set optional `Last-Modified`
    fn set_opt_last_modified(&mut self, time: Option<String>) -> Result<(), InvalidHeaderValue>;

    /// set `Content-Type` by mime
    fn set_mime(&mut self, mime: &Mime) -> Result<(), InvalidHeaderValue>;
}

impl ResponseExt for Response {
    fn new_with_status(body: impl Into<Body>, status: StatusCode) -> Self {
        let mut res = Self::new(body.into());
        *res.status_mut() = status;
        res
    }

    fn set_opt_header(
        &mut self,
        name: HeaderName,
        value: Option<String>,
    ) -> Result<(), InvalidHeaderValue> {
        if let Some(value) = value {
            let val = HeaderValue::try_from(value)?;
            let _ = self.headers_mut().insert(name, val);
        }
        Ok(())
    }

    fn set_opt_last_modified(&mut self, time: Option<String>) -> Result<(), InvalidHeaderValue> {
        self.set_opt_header(header::LAST_MODIFIED, time)
    }

    fn set_mime(&mut self, mime: &Mime) -> Result<(), InvalidHeaderValue> {
        let val = HeaderValue::try_from(mime.as_ref())?;
        let _ = self.headers_mut().insert(header::CONTENT_TYPE, val);
        Ok(())
    }
}
