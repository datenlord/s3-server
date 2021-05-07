//! [`DeleteObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest};
use crate::errors::{S3Error, S3Result};
use crate::headers::{
    X_AMZ_BYPASS_GOVERNANCE_RETENTION, X_AMZ_DELETE_MARKER, X_AMZ_MFA, X_AMZ_REQUEST_CHARGED,
    X_AMZ_REQUEST_PAYER, X_AMZ_VERSION_ID,
};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::ResponseExt;
use crate::{async_trait, Method, Response, StatusCode};

/// `DeleteObject` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::DELETE);
        ctx.path.is_object()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.delete_object(input).await;
        output.try_into_response()
    }
}

/// extract operation request
fn extract(ctx: &mut ReqContext<'_>) -> S3Result<DeleteObjectRequest> {
    let (bucket, key) = ctx.unwrap_object_path();

    let mut input: DeleteObjectRequest = DeleteObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        ..DeleteObjectRequest::default()
    };

    let h = &ctx.headers;

    h.assign(
        &*X_AMZ_BYPASS_GOVERNANCE_RETENTION,
        &mut input.bypass_governance_retention,
    )
    .map_err(|err| invalid_request!("Invalid header: x-amz-bypass-governance-retention", err))?;

    h.assign_str(&*X_AMZ_MFA, &mut input.mfa);
    h.assign_str(&*X_AMZ_REQUEST_PAYER, &mut input.request_payer);

    if let Some(ref qs) = ctx.query_strings {
        input.version_id = qs.get("versionId").map(ToOwned::to_owned);
    }

    Ok(input)
}

impl S3Output for DeleteObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_status(StatusCode::NO_CONTENT);
            res.set_optional_header(
                &*X_AMZ_DELETE_MARKER,
                self.delete_marker.map(|b| b.to_string()),
            )?;
            res.set_optional_header(&*X_AMZ_VERSION_ID, self.version_id)?;
            res.set_optional_header(&*X_AMZ_REQUEST_CHARGED, self.request_charged)?;
            Ok(())
        })
    }
}

impl From<DeleteObjectError> for S3Error {
    fn from(e: DeleteObjectError) -> Self {
        match e {}
    }
}
