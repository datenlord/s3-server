//! [`PutObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)

use super::*;
use crate::dto::{ByteStream, PutObjectError, PutObjectOutput, PutObjectRequest};
use futures::stream::StreamExt;
use std::io;

/// extract operation request
#[allow(clippy::cognitive_complexity)]
pub fn extract(
    req: &Request,
    body: Body,
    bucket: &str,
    key: &str,
) -> Result<PutObjectRequest, BoxStdError> {
    let body = body
        .map(|try_chunk| {
            try_chunk.map(|c| c).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error obtaining chunk: {}", e),
                )
            })
        })
        .apply(ByteStream::new);

    let mut input: PutObjectRequest = PutObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        body: Some(body),
        ..PutObjectRequest::default()
    };

    if let Some(content_length) = req.get_header_str(CONTENT_LENGTH)? {
        input.content_length = content_length.parse::<i64>()?.apply(Some)
    }

    assign_opt!(from req to input headers [
        &*X_AMZ_ACL => acl,
        CACHE_CONTROL => cache_control,
        CONTENT_DISPOSITION => content_disposition,
        CONTENT_ENCODING => content_encoding,
        CONTENT_LANGUAGE => content_language,
        &*CONTENT_MD5 => content_md5,
        CONTENT_TYPE => content_type,
        EXPIRES => expires,
        &*X_AMZ_GRANT_FULL_CONTROL => grant_full_control,
        &*X_AMZ_GRANT_READ => grant_read,
        &*X_AMZ_GRANT_READ_ACP => grant_read_acp,
        &*X_AMZ_GRANT_WRITE_ACP => grant_write_acp,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION => server_side_encryption,
        &*X_AMZ_STORAGE_CLASS => storage_class,
        &*X_AMZ_WEBSITE_REDIRECT_LOCATION => website_redirect_location,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM => sse_customer_algorithm,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY => sse_customer_key,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5 => sse_customer_key_md5,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID => ssekms_key_id,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT => ssekms_encryption_context,
        &*X_AMZ_REQUEST_PAYER => request_payer,
        &*X_AMZ_TAGGING => tagging,
        &*X_AMZ_OBJECT_LOCK_MODE => object_lock_mode,
        &*X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE => object_lock_retain_until_date,
        &*X_AMZ_OBJECT_LOCK_LEGAL_HOLD => object_lock_legal_hold_status,
    ]);

    Ok(input)
}

impl S3Output for PutObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_opt_header(|| X_AMZ_EXPIRATION.clone(), self.expiration)?;
            res.set_opt_header(|| ETAG, self.e_tag)?;
            res.set_opt_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION.clone(),
                self.server_side_encryption,
            )?;
            res.set_opt_header(|| X_AMZ_VERSION_ID.clone(), self.version_id)?;
            res.set_opt_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM.clone(),
                self.sse_customer_algorithm,
            )?;
            res.set_opt_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5.clone(),
                self.sse_customer_key_md5,
            )?;
            res.set_opt_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID.clone(),
                self.ssekms_key_id,
            )?;
            res.set_opt_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT.clone(),
                self.ssekms_encryption_context,
            )?;
            res.set_opt_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;
            Ok(())
        })
    }
}

impl S3Output for PutObjectError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}
