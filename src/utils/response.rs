//! response util

use crate::{BoxStdError, Response};
use hyper::{
    header::{self, HeaderName, HeaderValue, InvalidHeaderValue},
    Body, StatusCode,
};
use mime::Mime;
use std::convert::TryFrom;
use xml::{common::XmlVersion, writer::XmlEvent, EventWriter};

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

    /// set xml body
    fn set_xml_body<F>(&mut self, cap: usize, f: F) -> Result<(), BoxStdError>
    where
        F: FnOnce(&mut EventWriter<&mut Vec<u8>>) -> Result<(), xml::writer::Error>;
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

    /// set xml body
    fn set_xml_body<F>(&mut self, cap: usize, f: F) -> Result<(), BoxStdError>
    where
        F: FnOnce(&mut EventWriter<&mut Vec<u8>>) -> Result<(), xml::writer::Error>,
    {
        let mut body = Vec::with_capacity(cap);
        {
            let mut w = EventWriter::new(&mut body);
            w.write(XmlEvent::StartDocument {
                version: XmlVersion::Version10,
                encoding: Some("UTF-8"),
                standalone: None,
            })?;

            f(&mut w)?;
        }

        *self.body_mut() = Body::from(body);
        self.set_mime(&mime::TEXT_XML)?;
        Ok(())
    }
}
