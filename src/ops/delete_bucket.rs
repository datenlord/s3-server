//! [`DeleteBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteBucket.html)

use super::*;
use crate::dto::{DeleteBucketError, DeleteBucketOutput};

impl S3Output for DeleteBucketOutput {
    fn try_into_response(self) -> S3Result<Response> {
        Response::new_with_status(Body::empty(), StatusCode::NO_CONTENT).apply(Ok)
    }
}

impl S3Output for DeleteBucketError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}
