//! [`CreateBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateBucket.html)

use super::*;
use crate::dto::{CreateBucketError, CreateBucketOutput};

impl S3Output for CreateBucketOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_opt_header(|| LOCATION, self.location)?;
            Ok(())
        })
    }
}

impl S3Output for CreateBucketError {
    fn try_into_response(self) -> S3Result<Response> {
        let resp = match self {
            Self::BucketAlreadyExists(msg) => {
                XmlErrorResponse::from_code_msg(S3ErrorCode::BucketAlreadyExists, msg)
            }
            Self::BucketAlreadyOwnedByYou(msg) => {
                XmlErrorResponse::from_code_msg(S3ErrorCode::BucketAlreadyOwnedByYou, msg)
            }
        };
        resp.try_into_response()
    }
}
