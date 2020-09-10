//! [`GetObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html)

use super::*;
use crate::dto::{GetObjectError, GetObjectOutput, GetObjectRequest};

/// extract operation request
pub fn extract(req: &Request, bucket: &str, key: &str) -> Result<GetObjectRequest, BoxStdError> {
    let mut input = GetObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        ..GetObjectRequest::default()
    };

    assign_opt!(from req to input headers [
        IF_MATCH => if_match,
        IF_MODIFIED_SINCE => if_modified_since,
        IF_NONE_MATCH => if_none_match,
        IF_UNMODIFIED_SINCE => if_unmodified_since,
        RANGE => range,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM => sse_customer_algorithm,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY => sse_customer_key,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5 => sse_customer_key_md5,
        &*X_AMZ_REQUEST_PAYER => request_payer,
    ]);

    Ok(input)
}

impl S3Output for GetObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_opt_header(
                || X_AMZ_DELETE_MARKER.clone(),
                self.delete_marker.map(|b| b.to_string()),
            )?;

            res.set_opt_header(|| ACCEPT_RANGES, self.accept_ranges)?;

            res.set_opt_header(|| X_AMZ_EXPIRATION.clone(), self.expiration)?;
            res.set_opt_header(|| X_AMZ_RESTORE.clone(), self.restore)?;

            res.set_opt_header(
                || LAST_MODIFIED,
                time::map_opt_rfc3339_to_last_modified(self.last_modified)?,
            )?;

            res.set_opt_header(
                || CONTENT_LENGTH,
                self.content_length.map(|l| l.to_string()),
            )?;

            res.set_opt_header(|| ETAG, self.e_tag)?;

            res.set_opt_header(
                || X_AMZ_MISSING_META.clone(),
                self.missing_meta.map(|m| m.to_string()),
            )?;

            res.set_opt_header(|| X_AMZ_VERSION_ID.clone(), self.version_id)?;
            res.set_opt_header(|| CACHE_CONTROL, self.cache_control)?;

            res.set_opt_header(|| CONTENT_DISPOSITION, self.content_disposition)?;
            res.set_opt_header(|| CONTENT_ENCODING, self.content_encoding)?;
            res.set_opt_header(|| CONTENT_LANGUAGE, self.content_language)?;
            res.set_opt_header(|| CONTENT_RANGE, self.content_range)?;
            res.set_opt_header(|| CONTENT_TYPE, self.content_type)?;

            res.set_opt_header(|| EXPIRES, self.expires)?;

            res.set_opt_header(
                || X_AMZ_WEBSITE_REDIRECT_LOCATION.clone(),
                self.website_redirect_location,
            )?;

            res.set_opt_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION.clone(),
                self.server_side_encryption,
            )?;
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

            res.set_opt_header(|| X_AMZ_STORAGE_CLASS.clone(), self.storage_class)?;
            res.set_opt_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;
            res.set_opt_header(|| X_AMZ_REPLICATION_STATUS.clone(), self.replication_status)?;
            res.set_opt_header(
                || X_AMZ_MP_PARTS_COUNT.clone(),
                self.parts_count.map(|c| c.to_string()),
            )?;
            res.set_opt_header(
                || X_AMZ_TAGGING_COUNT.clone(),
                self.tag_count.map(|c| c.to_string()),
            )?;
            res.set_opt_header(|| X_AMZ_OBJECT_LOCK_MODE.clone(), self.object_lock_mode)?;
            res.set_opt_header(
                || X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE.clone(),
                self.object_lock_retain_until_date,
            )?;
            res.set_opt_header(
                || X_AMZ_OBJECT_LOCK_LEGAL_HOLD.clone(),
                self.object_lock_legal_hold_status,
            )?;

            if let Some(body) = self.body {
                *res.body_mut() = Body::wrap_stream(body);
            }

            Ok(())
        })
    }
}

impl S3Output for GetObjectError {
    fn try_into_response(self) -> S3Result<Response> {
        let resp = match self {
            Self::NoSuchKey(msg) => XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchKey, msg),
        };
        resp.try_into_response()
    }
}
