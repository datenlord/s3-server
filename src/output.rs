//! Types which can be converted into a response

use crate::error::{InvalidOutputError, S3Error, S3Result};
use crate::error_code::S3ErrorCode;
use crate::utils::{create_response, set_mime, xml_write_string_element, Response};

use std::convert::TryFrom;

use hyper::{
    header::{self, HeaderValue},
    Body, StatusCode,
};
use xml::{
    common::XmlVersion,
    writer::{EventWriter, XmlEvent},
};

use crate::dto::{
    CreateBucketError, CreateBucketOutput, DeleteBucketError, DeleteObjectError,
    DeleteObjectOutput, GetBucketLocationOutput, GetObjectError, GetObjectOutput, HeadBucketError,
    ListBucketsError, ListBucketsOutput, PutObjectError, PutObjectOutput,
};

/// Types which can be converted into a response
pub trait S3Output {
    /// Try to convert into a response
    /// # Errors
    /// Returns an `Err` if the output can not be converted to a response
    fn try_into_response(self) -> S3Result<Response>;
}

impl<T, E> S3Output for S3Result<T, E>
where
    T: S3Output,
    E: S3Output,
{
    fn try_into_response(self) -> S3Result<Response> {
        match self {
            Ok(output) => output.try_into_response(),
            Err(err) => match err {
                S3Error::Operation(e) => e.try_into_response(),
                S3Error::InvalidRequest(e) => Err(<S3Error>::InvalidRequest(e)),
                S3Error::InvalidOutput(e) => Err(<S3Error>::InvalidOutput(e)),
                S3Error::Storage(e) => Err(<S3Error>::Storage(e)),
                S3Error::NotSupported => Err(S3Error::NotSupported),
            },
        }
    }
}

/// helper function for error converting
fn wrap_output(f: impl FnOnce() -> Result<Response, InvalidOutputError>) -> S3Result<Response> {
    match f() {
        Ok(res) => Ok(res),
        Err(e) => Err(<S3Error>::InvalidOutput(e)),
    }
}

impl S3Output for GetObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|| {
            let mut res = Response::new(Body::empty());
            if let Some(body) = self.body {
                *res.body_mut() = Body::wrap_stream(body);
            }
            if let Some(content_length) = self.content_length {
                let val = HeaderValue::try_from(format!("{}", content_length))?;
                let _ = res.headers_mut().insert(header::CONTENT_LENGTH, val);
            }
            if let Some(content_type) = self.content_type {
                let val = HeaderValue::try_from(content_type)?;
                let _ = res.headers_mut().insert(header::CONTENT_TYPE, val);
            }
            // TODO: handle other fields
            Ok(res)
        })
    }
}

impl S3Output for CreateBucketOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|| {
            let mut res = Response::new(Body::empty());
            if let Some(location) = self.location {
                let val = HeaderValue::try_from(location)?;
                let _ = res.headers_mut().insert(header::LOCATION, val);
            }
            Ok(res)
        })
    }
}

impl S3Output for PutObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        let res = Response::new(Body::empty());
        // TODO: handle other fields
        Ok(res)
    }
}

impl S3Output for () {
    fn try_into_response(self) -> S3Result<Response> {
        let res = Response::new(Body::empty());
        Ok(res)
    }
}

impl S3Output for DeleteObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        let res = Response::new(Body::empty());
        // TODO: handle other fields
        Ok(res)
    }
}

impl S3Output for ListBucketsOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|| {
            let mut body = Vec::with_capacity(4096);
            {
                let mut w = EventWriter::new(&mut body);
                w.write(XmlEvent::StartDocument {
                    version: XmlVersion::Version10,
                    encoding: Some("UTF-8"),
                    standalone: None,
                })?;

                w.write(XmlEvent::start_element("ListBucketsOutput"))?;

                if let Some(buckets) = self.buckets {
                    w.write(XmlEvent::start_element("Buckets"))?;

                    for bucket in buckets {
                        w.write(XmlEvent::start_element("Bucket"))?;
                        if let Some(creation_date) = bucket.creation_date {
                            xml_write_string_element(&mut w, "CreationDate", &creation_date)?;
                        }

                        if let Some(name) = bucket.name {
                            xml_write_string_element(&mut w, "Name", &name)?;
                        }
                        w.write(XmlEvent::end_element())?;
                    }

                    w.write(XmlEvent::end_element())?;
                }

                if let Some(owner) = self.owner {
                    w.write(XmlEvent::start_element("Owner"))?;
                    if let Some(display_name) = owner.display_name {
                        xml_write_string_element(&mut w, "DisplayName", &display_name)?;
                    }
                    if let Some(id) = owner.id {
                        xml_write_string_element(&mut w, "ID", &id)?;
                    }
                }

                w.write(XmlEvent::end_element())?;
            }

            let mut res = Response::new(Body::from(body));
            set_mime(&mut res, &mime::TEXT_XML)?;

            // TODO: handle other fields

            Ok(res)
        })
    }
}

impl S3Output for GetBucketLocationOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|| {
            let mut body = Vec::with_capacity(64);
            let mut w = EventWriter::new(&mut body);
            w.write(XmlEvent::StartDocument {
                version: XmlVersion::Version10,
                encoding: Some("UTF-8"),
                standalone: None,
            })?;

            w.write(XmlEvent::start_element("LocationConstraint"))?;
            if let Some(location_constraint) = self.location_constraint {
                w.write(XmlEvent::characters(&location_constraint))?;
            }
            w.write(XmlEvent::end_element())?;

            let mut res = Response::new(Body::from(body));
            set_mime(&mut res, &mime::TEXT_XML)?;
            // TODO: handle other fields

            Ok(res)
        })
    }
}

/// Type representing an error response
#[derive(Debug)]
struct XmlErrorResponse {
    /// code
    code: S3ErrorCode,
    /// message
    message: Option<String>,
    /// resource
    resource: Option<String>,
    /// request_id
    request_id: Option<String>,
}

impl XmlErrorResponse {
    /// Constructs a `XmlErrorResponse`
    const fn from_code_msg(code: S3ErrorCode, message: Option<String>) -> Self {
        Self {
            code,
            message,
            resource: None,
            request_id: None,
        }
    }
}

impl S3Output for XmlErrorResponse {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|| {
            let mut body = Vec::with_capacity(64);
            let mut w = EventWriter::new(&mut body);
            w.write(XmlEvent::StartDocument {
                version: XmlVersion::Version10,
                encoding: Some("UTF-8"),
                standalone: None,
            })?;

            w.write(XmlEvent::start_element("Error"))?;

            xml_write_string_element(&mut w, "Code", &self.code.to_string())?;
            if let Some(message) = self.message {
                xml_write_string_element(&mut w, "Message", &message)?;
            }
            if let Some(resource) = self.resource {
                xml_write_string_element(&mut w, "Resource", &resource)?;
            }
            if let Some(request_id) = self.request_id {
                xml_write_string_element(&mut w, "RequestId", &request_id)?;
            }

            w.write(XmlEvent::end_element())?;

            let status = self
                .code
                .as_status_code()
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            let mut res = create_response(body, Some(status));
            set_mime(&mut res, &mime::TEXT_XML)?;

            Ok(res)
        })
    }
}

impl S3Output for HeadBucketError {
    fn try_into_response(self) -> S3Result<Response> {
        let resp = match self {
            Self::NoSuchBucket(msg) => {
                XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchBucket, msg.into())
            }
        };
        resp.try_into_response()
    }
}

impl S3Output for ListBucketsError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}

impl S3Output for PutObjectError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}

impl S3Output for DeleteObjectError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}

impl S3Output for DeleteBucketError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}

impl S3Output for CreateBucketError {
    fn try_into_response(self) -> S3Result<Response> {
        let resp = match self {
            Self::BucketAlreadyExists(msg) => {
                XmlErrorResponse::from_code_msg(S3ErrorCode::BucketAlreadyExists, msg.into())
            }
            Self::BucketAlreadyOwnedByYou(msg) => {
                XmlErrorResponse::from_code_msg(S3ErrorCode::BucketAlreadyOwnedByYou, msg.into())
            }
        };
        resp.try_into_response()
    }
}

impl S3Output for GetObjectError {
    fn try_into_response(self) -> S3Result<Response> {
        let resp = match self {
            Self::NoSuchKey(msg) => {
                XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchKey, msg.into())
            }
        };
        resp.try_into_response()
    }
}
