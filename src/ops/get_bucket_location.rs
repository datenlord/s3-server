//! [`GetBucketLocation`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetBucketLocation.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{GetBucketLocationError, GetBucketLocationOutput, GetBucketLocationRequest};
use crate::errors::{S3Error, S3Result};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::{Apply, ResponseExt, XmlWriterExt};
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

    GetBucketLocationRequest {
        bucket: bucket.into(),
    }
    .apply(Ok)
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
