//! [`CreateBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateBucket.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{
    CreateBucketConfiguration, CreateBucketError, CreateBucketOutput, CreateBucketRequest,
};
use crate::errors::{S3Error, S3ErrorCode, S3Result};
use crate::headers::{
    LOCATION, X_AMZ_ACL, X_AMZ_BUCKET_OBJECT_LOCK_ENABLED, X_AMZ_GRANT_FULL_CONTROL,
    X_AMZ_GRANT_READ, X_AMZ_GRANT_READ_ACP, X_AMZ_GRANT_WRITE, X_AMZ_GRANT_WRITE_ACP,
};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::{deserialize_xml_body, ResponseExt};
use crate::{async_trait, Method, Response};

/// `CreateBucket` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::PUT);
        ctx.path.is_bucket()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx).await?;
        let output = storage.create_bucket(input).await;
        output.try_into_response()
    }
}

/// extract operation request
async fn extract(ctx: &mut ReqContext<'_>) -> S3Result<CreateBucketRequest> {
    let bucket = ctx.unwrap_bucket_path();

    let config: Option<self::xml::CreateBucketConfiguration> =
        deserialize_xml_body(ctx.take_body())
            .await
            .map_err(|err| invalid_request!("Invalid xml format", err))?;

    let mut input: CreateBucketRequest = CreateBucketRequest {
        bucket: bucket.into(),
        create_bucket_configuration: config.map(Into::into),
        ..CreateBucketRequest::default()
    };

    let h = &ctx.headers;
    h.assign_str(&*X_AMZ_ACL, &mut input.acl);
    h.assign_str(&*X_AMZ_GRANT_FULL_CONTROL, &mut input.grant_full_control);
    h.assign_str(&*X_AMZ_GRANT_READ, &mut input.grant_read);
    h.assign_str(&*X_AMZ_GRANT_READ_ACP, &mut input.grant_read_acp);
    h.assign_str(&*X_AMZ_GRANT_WRITE, &mut input.grant_write);
    h.assign_str(&*X_AMZ_GRANT_WRITE_ACP, &mut input.grant_write_acp);
    h.assign(
        &*X_AMZ_BUCKET_OBJECT_LOCK_ENABLED,
        &mut input.object_lock_enabled_for_bucket,
    )
    .map_err(|err| invalid_request!("Invalid header: x-amz-bucket-object-lock-enabled", err))?;

    Ok(input)
}

impl S3Output for CreateBucketOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_optional_header(LOCATION, self.location)?;
            Ok(())
        })
    }
}

impl From<CreateBucketError> for S3Error {
    fn from(e: CreateBucketError) -> Self {
        match e {
            CreateBucketError::BucketAlreadyExists(msg) => {
                Self::new(S3ErrorCode::BucketAlreadyExists, msg)
            }
            CreateBucketError::BucketAlreadyOwnedByYou(msg) => {
                Self::new(S3ErrorCode::BucketAlreadyOwnedByYou, msg)
            }
        }
    }
}

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
