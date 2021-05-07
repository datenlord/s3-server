//! response util

use crate::{Body, BoxStdError, Mime, Response, StatusCode};

use std::{collections::HashMap, convert::TryFrom};

use hyper::header::{self, HeaderName, HeaderValue, InvalidHeaderValue};
use xml::{common::XmlVersion, writer::XmlEvent, EventWriter};

/// `ResponseExt`
pub trait ResponseExt {
    /// create response with body and status
    fn new_with_status(body: impl Into<Body>, status: StatusCode) -> Self;

    /// set optional header
    fn set_optional_header(
        &mut self,
        name: impl HeaderExt,
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

    /// set metadata headers
    fn set_metadata_headers(
        &mut self,
        metadata: &HashMap<String, String>,
    ) -> Result<(), BoxStdError>;
}

impl ResponseExt for Response {
    fn new_with_status(body: impl Into<Body>, status: StatusCode) -> Self {
        let mut res = Self::new(body.into());
        *res.status_mut() = status;
        res
    }

    fn set_optional_header(
        &mut self,
        name: impl HeaderExt,
        value: Option<String>,
    ) -> Result<(), InvalidHeaderValue> {
        if let Some(value) = value {
            let val = HeaderValue::try_from(value)?;
            let _prev = self.headers_mut().insert(name.into_owned_name(), val);
        }
        Ok(())
    }

    fn set_mime(&mut self, mime: &Mime) -> Result<(), InvalidHeaderValue> {
        let val = HeaderValue::try_from(mime.as_ref())?;
        let _prev = self.headers_mut().insert(header::CONTENT_TYPE, val);
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

    fn set_metadata_headers(
        &mut self,
        metadata: &HashMap<String, String>,
    ) -> Result<(), BoxStdError> {
        let headers = self.headers_mut();
        for (name, value) in metadata {
            let header_name = HeaderName::from_bytes(format!("x-amz-meta-{}", name).as_bytes())?;
            let header_value = HeaderValue::from_bytes(value.as_bytes())?;
            let _prev = headers.insert(header_name, header_value);
        }
        Ok(())
    }
}

/// header ext
pub trait HeaderExt {
    /// into owned name
    fn into_owned_name(self) -> HeaderName;
}

impl HeaderExt for &'_ HeaderName {
    fn into_owned_name(self) -> HeaderName {
        self.clone()
    }
}

impl HeaderExt for HeaderName {
    fn into_owned_name(self) -> HeaderName {
        self
    }
}
