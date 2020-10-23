//! [`DeleteBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteBucket.html)

use super::{ReqContext, S3Handler};

use crate::dto::{DeleteBucketError, DeleteBucketOutput, DeleteBucketRequest};
use crate::errors::{S3Error, S3Result};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::{Apply, ResponseExt};
use crate::{async_trait, Body, Method, Response, StatusCode};

/// `DeleteBucket` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::DELETE);
        ctx.path.is_bucket()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.delete_bucket(input).await;
        output.try_into_response()
    }
}

/// extract operation request
fn extract(ctx: &mut ReqContext<'_>) -> S3Result<DeleteBucketRequest> {
    let bucket = ctx.unwrap_bucket_path();
    DeleteBucketRequest {
        bucket: bucket.into(),
    }
    .apply(Ok)
}

impl S3Output for DeleteBucketOutput {
    fn try_into_response(self) -> S3Result<Response> {
        Response::new_with_status(Body::empty(), StatusCode::NO_CONTENT).apply(Ok)
    }
}

impl From<DeleteBucketError> for S3Error {
    fn from(e: DeleteBucketError) -> Self {
        match e {}
    }
}
