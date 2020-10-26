//! [`UploadPart`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_UploadPart.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{UploadPartError, UploadPartOutput, UploadPartRequest};
use crate::errors::{S3Error, S3Result};
use crate::headers::{
    CONTENT_LENGTH, CONTENT_MD5, ETAG, X_AMZ_REQUEST_CHARGED, X_AMZ_SERVER_SIDE_ENCRYPTION,
    X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::body::transform_body_stream;
use crate::utils::ResponseExt;
use crate::{async_trait, Method, Response};

/// `UploadPart` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::PUT);
        let qs = bool_try_some!(ctx.query_strings.as_ref());
        qs.get("partNumber").is_some() && qs.get("uploadId").is_some()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.upload_part(input).await;
        output.try_into_response()
    }
}

/// extract operation request
fn extract(
    ctx: &mut ReqContext<'_>,
    // req: &Request,
    // bucket: &str,
    // key: &str,
    // part_number: i64,
    // upload_id: String,
    // body: Body,
) -> S3Result<UploadPartRequest> {
    let (bucket, key) = ctx.unwrap_object_path();

    let part_number = ctx
        .unwrap_qs("partNumber")
        .parse::<i64>()
        .map_err(|err| invalid_request!("Invalid query: partNumber", err))?;

    let upload_id = ctx.unwrap_qs("uploadId").to_owned();

    let body = transform_body_stream(ctx.take_body());

    let mut input = UploadPartRequest {
        bucket: bucket.into(),
        key: key.into(),
        part_number,
        upload_id,
        body: Some(body),
        ..UploadPartRequest::default()
    };

    let h = &ctx.headers;
    h.assign(CONTENT_LENGTH, &mut input.content_length)
        .map_err(|err| invalid_request!("Invalid header: content-length", err))?;
    h.assign_str(&*CONTENT_MD5, &mut input.content_md5);
    h.assign_str(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
        &mut input.sse_customer_algorithm,
    );
    h.assign_str(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
        &mut input.sse_customer_key,
    );
    h.assign_str(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
        &mut input.sse_customer_key_md5,
    );

    Ok(input)
}

impl S3Output for UploadPartOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_optional_header(ETAG, self.e_tag)?;

            res.set_optional_header(&*X_AMZ_SERVER_SIDE_ENCRYPTION, self.server_side_encryption)?;
            res.set_optional_header(
                &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
                self.sse_customer_algorithm,
            )?;
            res.set_optional_header(
                &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
                self.sse_customer_key_md5,
            )?;
            res.set_optional_header(
                &*X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
                self.ssekms_key_id,
            )?;

            res.set_optional_header(&*X_AMZ_REQUEST_CHARGED, self.request_charged)?;

            Ok(())
        })
    }
}

impl From<UploadPartError> for S3Error {
    fn from(e: UploadPartError) -> Self {
        match e {}
    }
}
