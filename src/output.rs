//! Types which can be converted into a response

use crate::{
    error::{S3Error, S3Result},
    error_code::S3ErrorCode,
    utils::{ResponseExt, XmlWriterExt},
    BoxStdError, Response,
};

use hyper::{Body, StatusCode};

/// Types which can be converted into a response
pub trait S3Output {
    /// Try to convert into a response
    ///
    /// # Errors
    /// Returns an `Err` if the output can not be converted into a response
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
pub(crate) fn wrap_output(
    f: impl FnOnce(&mut Response) -> Result<(), BoxStdError>,
) -> S3Result<Response> {
    let mut res = Response::new(Body::empty());
    match f(&mut res) {
        Ok(()) => Ok(res),
        Err(e) => Err(<S3Error>::InvalidOutput(e)),
    }
}

/// Type representing an error response
#[derive(Debug)]
pub(crate) struct XmlErrorResponse {
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
    pub(crate) const fn from_code_msg(code: S3ErrorCode, message: String) -> Self {
        Self {
            code,
            message: Some(message),
            resource: None,
            request_id: None,
        }
    }
}

impl S3Output for XmlErrorResponse {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            let status = self
                .code
                .as_status_code()
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            *res.status_mut() = status;

            res.set_xml_body(64, |w| {
                w.stack("Error", |w| {
                    w.element("Code", self.code.as_static_str())?;
                    w.opt_element("Message", self.message)?;
                    w.opt_element("Resource", self.resource)?;
                    w.opt_element("RequestId", self.request_id)?;
                    Ok(())
                })
            })?;

            Ok(())
        })
    }
}
