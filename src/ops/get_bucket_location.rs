//! [`GetBucketLocation`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetBucketLocation.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{GetBucketLocationError, GetBucketLocationOutput, GetBucketLocationRequest};
use crate::errors::{S3Error, S3Result};
use crate::headers::X_AMZ_EXPECTED_BUCKET_OWNER;
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::{ResponseExt, XmlWriterExt};
use crate::{async_trait, Method, Response};

/// `GetBucketLocation` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::GET);
        bool_try!(ctx.path.is_bucket());
        let qs = bool_try_some!(ctx.query_strings.as_ref());
        qs.get("location").is_some()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.get_bucket_location(input).await;
        output.try_into_response()
    }
}

/// extract operation request
fn extract(ctx: &mut ReqContext<'_>) -> S3Result<GetBucketLocationRequest> {
    let bucket = ctx.unwrap_bucket_path();

    let mut input = GetBucketLocationRequest {
        bucket: bucket.into(),
        expected_bucket_owner: None,
    };

    let h = &ctx.headers;
    h.assign_str(
        &*X_AMZ_EXPECTED_BUCKET_OWNER,
        &mut input.expected_bucket_owner,
    );

    Ok(input)
}

impl S3Output for GetBucketLocationOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_xml_body(4096, |w| {
                w.element(
                    "LocationConstraint",
                    self.location_constraint.as_deref().unwrap_or(""),
                )
            })
        })
    }
}

impl From<GetBucketLocationError> for S3Error {
    fn from(e: GetBucketLocationError) -> Self {
        match e {}
    }
}
