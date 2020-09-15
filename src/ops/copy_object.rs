//! [`CopyObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CopyObject.html)

use crate::error::S3Result;
use crate::error_code::S3ErrorCode;
use crate::output::{wrap_output, S3Output, XmlErrorResponse};
use crate::utils::{RequestExt, ResponseExt, XmlWriterExt};
use crate::{BoxStdError, Request, Response};

use crate::dto::{CopyObjectError, CopyObjectOutput, CopyObjectRequest};
use crate::header::names::{
    X_AMZ_ACL, X_AMZ_COPY_SOURCE_IF_MATCH, X_AMZ_COPY_SOURCE_IF_MODIFIED_SINCE,
    X_AMZ_COPY_SOURCE_IF_NONE_MATCH, X_AMZ_COPY_SOURCE_IF_UNMODIFIED_SINCE,
    X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
    X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
    X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5, X_AMZ_COPY_SOURCE_VERSION_ID,
    X_AMZ_EXPIRATION, X_AMZ_GRANT_FULL_CONTROL, X_AMZ_GRANT_READ, X_AMZ_GRANT_READ_ACP,
    X_AMZ_GRANT_WRITE_ACP, X_AMZ_METADATA_DIRECTIVE, X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
    X_AMZ_OBJECT_LOCK_MODE, X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE, X_AMZ_REQUEST_CHARGED,
    X_AMZ_REQUEST_PAYER, X_AMZ_SERVER_SIDE_ENCRYPTION, X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
    X_AMZ_STORAGE_CLASS, X_AMZ_TAGGING, X_AMZ_TAGGING_DIRECTIVE, X_AMZ_VERSION_ID,
    X_AMZ_WEBSITE_REDIRECT_LOCATION,
};
use hyper::header::{
    CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_TYPE, EXPIRES,
};

/// extract operation request
pub fn extract(
    req: &Request,
    bucket: &str,
    key: &str,
    copy_source: &str,
) -> Result<CopyObjectRequest, BoxStdError> {
    crate::header::CopySource::try_match(copy_source)?;

    let mut input: CopyObjectRequest = CopyObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        copy_source: copy_source.into(),
        ..CopyObjectRequest::default()
    };

    assign_opt!(from req to input headers [
        &*X_AMZ_ACL => acl,
        CACHE_CONTROL => cache_control,
        CONTENT_DISPOSITION => content_disposition,
        CONTENT_ENCODING => content_encoding,
        CONTENT_LANGUAGE => content_language,
        CONTENT_TYPE => content_type,
        &*X_AMZ_COPY_SOURCE_IF_MATCH => copy_source_if_match,
        &*X_AMZ_COPY_SOURCE_IF_MODIFIED_SINCE => copy_source_if_modified_since,
        &*X_AMZ_COPY_SOURCE_IF_NONE_MATCH => copy_source_if_none_match,
        &*X_AMZ_COPY_SOURCE_IF_UNMODIFIED_SINCE => copy_source_if_unmodified_since,
        EXPIRES => expires,
        &*X_AMZ_GRANT_FULL_CONTROL => grant_full_control,
        &*X_AMZ_GRANT_READ => grant_read,
        &*X_AMZ_GRANT_READ_ACP => grant_read_acp,
        &*X_AMZ_GRANT_WRITE_ACP => grant_write_acp,
        &*X_AMZ_METADATA_DIRECTIVE => metadata_directive,
        &*X_AMZ_TAGGING_DIRECTIVE => tagging_directive,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION => server_side_encryption,
        &*X_AMZ_STORAGE_CLASS => storage_class,
        &*X_AMZ_WEBSITE_REDIRECT_LOCATION => website_redirect_location,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM => sse_customer_algorithm,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY => sse_customer_key,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5 => sse_customer_key_md5,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID => ssekms_key_id,
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT => ssekms_encryption_context,
        &*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM => copy_source_sse_customer_algorithm,
        &*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY => copy_source_sse_customer_key,
        &*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5 => copy_source_sse_customer_key_md5,
        &*X_AMZ_REQUEST_PAYER => request_payer,
        &*X_AMZ_TAGGING => tagging,
        &*X_AMZ_OBJECT_LOCK_MODE => object_lock_mode,
        &*X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE => object_lock_retain_until_date,
        &*X_AMZ_OBJECT_LOCK_LEGAL_HOLD => object_lock_legal_hold_status,
    ]);

    Ok(input)
}

impl S3Output for CopyObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_opt_header(|| X_AMZ_EXPIRATION.clone(), self.expiration)?;
            res.set_opt_header(
                || X_AMZ_COPY_SOURCE_VERSION_ID.clone(),
                self.copy_source_version_id,
            )?;
            res.set_opt_header(|| X_AMZ_VERSION_ID.clone(), self.version_id)?;
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
            res.set_opt_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT.clone(),
                self.ssekms_encryption_context,
            )?;
            res.set_opt_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;

            let copy_object_result = self.copy_object_result;

            res.set_xml_body(64, |w| {
                w.opt_stack("CopyObjectResult", copy_object_result, |w, result| {
                    w.opt_element("ETag", result.e_tag)?;
                    w.opt_element("LastModified", result.last_modified)
                })
            })?;

            Ok(())
        })
    }
}

impl S3Output for CopyObjectError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {
            Self::ObjectNotInActiveTierError(msg) => {
                XmlErrorResponse::from_code_msg(S3ErrorCode::ObjectNotInActiveTierError, msg)
            }
        }
        .try_into_response()
    }
}
