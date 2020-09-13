//! [`DeleteObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)

use crate::error::S3Result;
use crate::output::{wrap_output, S3Output};
use crate::utils::{RequestExt, ResponseExt};
use crate::{BoxStdError, Request, Response};

use hyper::StatusCode;
use serde::Deserialize;

use crate::dto::{DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest};
use crate::header::names::{
    X_AMZ_BYPASS_GOVERNANCE_RETENTION, X_AMZ_DELETE_MARKER, X_AMZ_MFA, X_AMZ_REQUEST_CHARGED,
    X_AMZ_REQUEST_PAYER, X_AMZ_VERSION_ID,
};

#[derive(Debug, Deserialize)]
/// `DeleteObject` request query
struct Query {
    /// versionId
    #[serde(rename = "versionId")]
    version_id: Option<String>,
}

/// extract operation request
pub fn extract(req: &Request, bucket: &str, key: &str) -> Result<DeleteObjectRequest, BoxStdError> {
    let mut input: DeleteObjectRequest = DeleteObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        ..DeleteObjectRequest::default()
    };

    assign_opt!(from req to input headers [
        &*X_AMZ_BYPASS_GOVERNANCE_RETENTION => bypass_governance_retention,
        &*X_AMZ_MFA => mfa,
        &*X_AMZ_REQUEST_PAYER => request_payer,
    ]);

    if let Some(query) = req.extract_query::<Query>()? {
        assign_opt!(from query to input fields [
            version_id,
        ]);
    }

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
