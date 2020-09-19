//! [`CreateBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateBucket.html)

use crate::error::S3Result;
use crate::error_code::S3ErrorCode;
use crate::output::{wrap_output, S3Output, XmlErrorResponse};
use crate::utils::{deserialize_xml_body, RequestExt, ResponseExt};
use crate::{Body, BoxStdError, Request, Response};

use crate::dto::{
    CreateBucketConfiguration, CreateBucketError, CreateBucketOutput, CreateBucketRequest,
};
use crate::headers::names::{
    X_AMZ_ACL, X_AMZ_BUCKET_OBJECT_LOCK_ENABLED, X_AMZ_GRANT_FULL_CONTROL, X_AMZ_GRANT_READ,
    X_AMZ_GRANT_READ_ACP, X_AMZ_GRANT_WRITE, X_AMZ_GRANT_WRITE_ACP,
};
use hyper::header::LOCATION;

mod xml {
    //! xml repr

    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    /// `CreateBucketConfiguration`
    pub struct CreateBucketConfiguration {
        /// LocationConstraint
        #[serde(rename = "LocationConstraint")]
        pub location_constraint: Option<String>,
    }

    impl From<CreateBucketConfiguration> for super::CreateBucketConfiguration {
        fn from(config: CreateBucketConfiguration) -> Self {
            Self {
                location_constraint: config.location_constraint,
            }
        }
    }
}

/// extract operation request
pub async fn extract(
    req: &Request,
    body: Body,
    bucket: &str,
) -> Result<CreateBucketRequest, BoxStdError> {
    let config: Option<self::xml::CreateBucketConfiguration> = deserialize_xml_body(body).await?;

    let mut input: CreateBucketRequest = CreateBucketRequest {
        bucket: bucket.into(),
        create_bucket_configuration: config.map(Into::into),
        ..CreateBucketRequest::default()
    };

    assign_opt!(from req to input headers [
        &*X_AMZ_ACL => acl,
        &*X_AMZ_GRANT_FULL_CONTROL =>  grant_full_control,
        &*X_AMZ_GRANT_READ =>  grant_read,
        &*X_AMZ_GRANT_READ_ACP =>  grant_read_acp,
        &*X_AMZ_GRANT_WRITE => grant_write,
        &*X_AMZ_GRANT_WRITE_ACP =>  grant_write_acp,
        &*X_AMZ_BUCKET_OBJECT_LOCK_ENABLED => object_lock_enabled_for_bucket,
    ]);

    Ok(input)
}

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
