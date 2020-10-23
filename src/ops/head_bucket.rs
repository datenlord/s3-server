//! [`HeadBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadBucket.html)

use super::{ReqContext, S3Handler};

use crate::dto::{HeadBucketError, HeadBucketOutput, HeadBucketRequest};
use crate::errors::{S3Error, S3ErrorCode, S3Result};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::Apply;
use crate::{async_trait, Body, Method, Response};

/// `HeadBucket` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::HEAD);
        ctx.path.is_bucket()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.head_bucket(input).await;
        output.try_into_response()
    }
}

/// extract operation request
fn extract(ctx: &mut ReqContext<'_>) -> S3Result<HeadBucketRequest> {
    let bucket = ctx.unwrap_bucket_path();

    HeadBucketRequest {
        bucket: bucket.into(),
    }
    .apply(Ok)
}

impl S3Output for HeadBucketOutput {
    fn try_into_response(self) -> S3Result<Response> {
        Response::new(Body::empty()).apply(Ok)
    }
}

impl From<HeadBucketError> for S3Error {
    fn from(e: HeadBucketError) -> Self {
        match e {
            HeadBucketError::NoSuchBucket(msg) => Self::new(S3ErrorCode::NoSuchBucket, msg),
        }
    }
}
