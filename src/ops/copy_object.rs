//! [`CopyObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CopyObject.html)

use crate::error::S3Result;
use crate::error_code::S3ErrorCode;
use crate::output::{wrap_output, S3Output, XmlErrorResponse};
use crate::utils::{RequestExt, ResponseExt, XmlWriterExt};
use crate::{BoxStdError, Request, Response};

use crate::dto::{CopyObjectError, CopyObjectOutput, CopyObjectRequest};
use crate::headers::names::{
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
    crate::headers::AmzCopySource::try_match(copy_source)?;

    let mut input: CopyObjectRequest = CopyObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        copy_source: copy_source.into(),
        ..CopyObjectRequest::default()
    };

    req.assign_from_optional_header(&*X_AMZ_ACL, &mut input.acl)?;
    req.assign_from_optional_header(CACHE_CONTROL, &mut input.cache_control)?;
    req.assign_from_optional_header(CONTENT_DISPOSITION, &mut input.content_disposition)?;
    req.assign_from_optional_header(CONTENT_ENCODING, &mut input.content_encoding)?;
    req.assign_from_optional_header(CONTENT_LANGUAGE, &mut input.content_language)?;
    req.assign_from_optional_header(CONTENT_TYPE, &mut input.content_type)?;
    req.assign_from_optional_header(
        &*X_AMZ_COPY_SOURCE_IF_MATCH,
        &mut input.copy_source_if_match,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_COPY_SOURCE_IF_MODIFIED_SINCE,
        &mut input.copy_source_if_modified_since,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_COPY_SOURCE_IF_NONE_MATCH,
        &mut input.copy_source_if_none_match,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_COPY_SOURCE_IF_UNMODIFIED_SINCE,
        &mut input.copy_source_if_unmodified_since,
    )?;
    req.assign_from_optional_header(EXPIRES, &mut input.expires)?;
    req.assign_from_optional_header(&*X_AMZ_GRANT_FULL_CONTROL, &mut input.grant_full_control)?;
    req.assign_from_optional_header(&*X_AMZ_GRANT_READ, &mut input.grant_read)?;
    req.assign_from_optional_header(&*X_AMZ_GRANT_READ_ACP, &mut input.grant_read_acp)?;
    req.assign_from_optional_header(&*X_AMZ_GRANT_WRITE_ACP, &mut input.grant_write_acp)?;
    req.assign_from_optional_header(&*X_AMZ_METADATA_DIRECTIVE, &mut input.metadata_directive)?;
    req.assign_from_optional_header(&*X_AMZ_TAGGING_DIRECTIVE, &mut input.tagging_directive)?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION,
        &mut input.server_side_encryption,
    )?;
    req.assign_from_optional_header(&*X_AMZ_STORAGE_CLASS, &mut input.storage_class)?;
    req.assign_from_optional_header(
        &*X_AMZ_WEBSITE_REDIRECT_LOCATION,
        &mut input.website_redirect_location,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
        &mut input.sse_customer_algorithm,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
        &mut input.sse_customer_key,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
        &mut input.sse_customer_key_md5,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
        &mut input.ssekms_key_id,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT,
        &mut input.ssekms_encryption_context,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
        &mut input.copy_source_sse_customer_algorithm,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
        &mut input.copy_source_sse_customer_key,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
        &mut input.copy_source_sse_customer_key_md5,
    )?;
    req.assign_from_optional_header(&*X_AMZ_REQUEST_PAYER, &mut input.request_payer)?;
    req.assign_from_optional_header(&*X_AMZ_TAGGING, &mut input.tagging)?;
    req.assign_from_optional_header(&*X_AMZ_OBJECT_LOCK_MODE, &mut input.object_lock_mode)?;
    req.assign_from_optional_header(
        &*X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE,
        &mut input.object_lock_retain_until_date,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
        &mut input.object_lock_legal_hold_status,
    )?;

    Ok(input)
}

impl S3Output for CopyObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_optional_header(|| X_AMZ_EXPIRATION.clone(), self.expiration)?;
            res.set_optional_header(
                || X_AMZ_COPY_SOURCE_VERSION_ID.clone(),
                self.copy_source_version_id,
            )?;
            res.set_optional_header(|| X_AMZ_VERSION_ID.clone(), self.version_id)?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION.clone(),
                self.server_side_encryption,
            )?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM.clone(),
                self.sse_customer_algorithm,
            )?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5.clone(),
                self.sse_customer_key_md5,
            )?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID.clone(),
                self.ssekms_key_id,
            )?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT.clone(),
                self.ssekms_encryption_context,
            )?;
            res.set_optional_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;

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
