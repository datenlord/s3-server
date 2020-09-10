//! [`HeadBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadBucket.html)

use super::*;
use crate::dto::{HeadBucketError, HeadBucketOutput, HeadBucketRequest};

/// extract operation request
pub fn extract(bucket: &str) -> Result<HeadBucketRequest, BoxStdError> {
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

impl S3Output for HeadBucketError {
    fn try_into_response(self) -> S3Result<Response> {
        let resp = match self {
            Self::NoSuchBucket(msg) => {
                XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchBucket, msg)
            }
        };
        resp.try_into_response()
    }
}
