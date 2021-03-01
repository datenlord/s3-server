//! [`GetObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{GetObjectError, GetObjectOutput, GetObjectRequest};
use crate::errors::{S3Error, S3ErrorCode, S3Result};
use crate::headers::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_LANGUAGE,
    CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, EXPIRES, IF_MATCH, IF_MODIFIED_SINCE,
    IF_NONE_MATCH, IF_UNMODIFIED_SINCE, LAST_MODIFIED, RANGE, X_AMZ_DELETE_MARKER,
    X_AMZ_EXPIRATION, X_AMZ_MISSING_META, X_AMZ_MP_PARTS_COUNT, X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
    X_AMZ_OBJECT_LOCK_MODE, X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE, X_AMZ_REPLICATION_STATUS,
    X_AMZ_REQUEST_CHARGED, X_AMZ_REQUEST_PAYER, X_AMZ_RESTORE, X_AMZ_SERVER_SIDE_ENCRYPTION,
    X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
    X_AMZ_STORAGE_CLASS, X_AMZ_TAGGING_COUNT, X_AMZ_VERSION_ID, X_AMZ_WEBSITE_REDIRECT_LOCATION,
};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::{time, ResponseExt};
use crate::{async_trait, Body, Method, Response};

/// `GetObject` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::GET);
        ctx.path.is_object()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.get_object(input).await;
        output.try_into_response()
    }
}

/// extract operation request
fn extract(ctx: &mut ReqContext<'_>) -> S3Result<GetObjectRequest> {
    let (bucket, key) = ctx.unwrap_object_path();

    let mut input = GetObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        ..GetObjectRequest::default()
    };

    let h = &ctx.headers;
    h.assign_str(IF_MATCH, &mut input.if_match);
    h.assign_str(IF_MODIFIED_SINCE, &mut input.if_modified_since);
    h.assign_str(IF_NONE_MATCH, &mut input.if_none_match);
    h.assign_str(IF_UNMODIFIED_SINCE, &mut input.if_unmodified_since);
    h.assign_str(RANGE, &mut input.range);
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
    h.assign_str(&*X_AMZ_REQUEST_PAYER, &mut input.request_payer);

    Ok(input)
}

impl S3Output for GetObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_optional_header(
                &*X_AMZ_DELETE_MARKER,
                self.delete_marker.map(|b| b.to_string()),
            )?;

            res.set_optional_header(ACCEPT_RANGES, self.accept_ranges)?;

            res.set_optional_header(&*X_AMZ_EXPIRATION, self.expiration)?;
            res.set_optional_header(&*X_AMZ_RESTORE, self.restore)?;

            res.set_optional_header(
                LAST_MODIFIED,
                time::map_opt_rfc3339_to_last_modified(self.last_modified)?,
            )?;

            res.set_optional_header(CONTENT_LENGTH, self.content_length.map(|l| l.to_string()))?;

            res.set_optional_header(ETAG, self.e_tag)?;

            res.set_optional_header(
                &*X_AMZ_MISSING_META,
                self.missing_meta.map(|m| m.to_string()),
            )?;

            res.set_optional_header(&*X_AMZ_VERSION_ID, self.version_id)?;
            res.set_optional_header(CACHE_CONTROL, self.cache_control)?;

            res.set_optional_header(CONTENT_DISPOSITION, self.content_disposition)?;
            res.set_optional_header(CONTENT_ENCODING, self.content_encoding)?;
            res.set_optional_header(CONTENT_LANGUAGE, self.content_language)?;
            res.set_optional_header(CONTENT_RANGE, self.content_range)?;
            res.set_optional_header(CONTENT_TYPE, self.content_type)?;

            res.set_optional_header(EXPIRES, self.expires)?;

            res.set_optional_header(
                &*X_AMZ_WEBSITE_REDIRECT_LOCATION,
                self.website_redirect_location,
            )?;

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

            res.set_optional_header(&*X_AMZ_STORAGE_CLASS, self.storage_class)?;
            res.set_optional_header(&*X_AMZ_REQUEST_CHARGED, self.request_charged)?;
            res.set_optional_header(&*X_AMZ_REPLICATION_STATUS, self.replication_status)?;
            res.set_optional_header(
                &*X_AMZ_MP_PARTS_COUNT,
                self.parts_count.map(|c| c.to_string()),
            )?;
            res.set_optional_header(&*X_AMZ_TAGGING_COUNT, self.tag_count.map(|c| c.to_string()))?;
            res.set_optional_header(&*X_AMZ_OBJECT_LOCK_MODE, self.object_lock_mode)?;
            res.set_optional_header(
                &*X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE,
                self.object_lock_retain_until_date,
            )?;
            res.set_optional_header(
                &*X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
                self.object_lock_legal_hold_status,
            )?;

            if let Some(ref metadata) = self.metadata {
                res.set_metadata_headers(metadata)?;
            }

            if let Some(body) = self.body {
                *res.body_mut() = Body::wrap_stream(body);
            }

            Ok(())
        })
    }
}

impl From<GetObjectError> for S3Error {
    fn from(e: GetObjectError) -> Self {
        match e {
            GetObjectError::NoSuchKey(msg) => Self::new(S3ErrorCode::NoSuchKey, msg),
            GetObjectError::InvalidObjectState(msg) => {
                Self::new(S3ErrorCode::InvalidObjectState, msg)
            }
        }
    }
}
