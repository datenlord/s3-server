//! [`GetBucketLocation`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetBucketLocation.html)

use super::*;
use crate::dto::{GetBucketLocationError, GetBucketLocationOutput};

impl S3Output for GetBucketLocationOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_xml_body(4096, |w| {
                w.element(
                    "LocationConstraint",
                    self.location_constraint.as_deref().unwrap_or(""),
                )
            })
        })
    }
}

impl S3Output for GetBucketLocationError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}
