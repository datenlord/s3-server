//! [`DeleteObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)

use super::*;
use crate::dto::{DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest};

/// extract operation request
pub fn extract(
    _req: &Request,
    bucket: &str,
    key: &str,
) -> Result<DeleteObjectRequest, BoxStdError> {
    let input: DeleteObjectRequest = DeleteObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        ..DeleteObjectRequest::default() // TODO: handle other fields
    };
    Ok(input)
}

impl S3Output for DeleteObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_status(StatusCode::NO_CONTENT);
            res.set_opt_header(
                || X_AMZ_DELETE_MARKER.clone(),
                self.delete_marker.map(|b| b.to_string()),
            )?;
            res.set_opt_header(|| X_AMZ_VERSION_ID.clone(), self.version_id)?;
            res.set_opt_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;
            Ok(())
        })
    }
}

impl S3Output for DeleteObjectError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}
