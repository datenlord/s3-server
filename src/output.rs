//! Types which can be converted into a response

use crate::errors::{S3Error, S3Result, S3StorageError, S3StorageResult, XmlErrorResponse};
use crate::utils::{ResponseExt, XmlWriterExt};
use crate::{Body, Response, StatusCode};

/// Types which can be converted into a response
pub trait S3Output {
    /// Try to convert into a response
    ///
    /// # Errors
    /// Returns an `Err` if the output can not be converted into a response
    fn try_into_response(self) -> S3Result<Response>;
}

impl<T, E> S3Output for S3StorageResult<T, E>
where
    T: S3Output,
    E: Into<S3Error>,
{
    fn try_into_response(self) -> S3Result<Response> {
        match self {
            Ok(output) => output.try_into_response(),
            Err(err) => match err {
                S3StorageError::Operation(e) => Err(e.into()),
                S3StorageError::Other(e) => Err(e),
            },
        }
    }
}

impl S3Output for XmlErrorResponse {
    fn try_into_response(self) -> S3Result<Response> {
        let status = self
            .code
            .as_status_code()
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let mut res = Response::new_with_status(Body::empty(), status);

        res.set_xml_body(64, |w| {
            w.stack("Error", |w| {
                w.element("Code", self.code.as_static_str())?;
                w.opt_element("Message", self.message)?;
                // w.opt_element("Resource", self.resource)?;
                // w.opt_element("RequestId", self.request_id)?;
                Ok(())
            })
        })
        .map_err(|e| internal_error!(e))?;

        Ok(res)
    }
}
