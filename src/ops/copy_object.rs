//! [`CopyObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CopyObject.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{CopyObjectError, CopyObjectOutput, CopyObjectRequest};
use crate::errors::{S3Error, S3ErrorCode, S3Result};
use crate::headers::AmzCopySource;
use crate::headers::{
    CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_TYPE, EXPIRES,
    X_AMZ_ACL, X_AMZ_COPY_SOURCE, X_AMZ_COPY_SOURCE_IF_MATCH, X_AMZ_COPY_SOURCE_IF_MODIFIED_SINCE,
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
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::{ResponseExt, XmlWriterExt};
use crate::{async_trait, Method, Response};

/// `CopyObject` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::PUT);
        bool_try!(ctx.path.is_object());
        ctx.headers.get(X_AMZ_COPY_SOURCE).is_some()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.copy_object(input).await;
        output.try_into_response()
    }
}

/// extract operation request
fn extract(ctx: &mut ReqContext<'_>) -> S3Result<CopyObjectRequest> {
    let (bucket, key) = ctx.unwrap_object_path();
    let copy_source = ctx.unwrap_header(X_AMZ_COPY_SOURCE);

    AmzCopySource::try_match(copy_source)
        .map_err(|err| invalid_request!("Invalid header: x-amz-copy-source", err))?;

    let mut input: CopyObjectRequest = CopyObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        copy_source: copy_source.into(),
        ..CopyObjectRequest::default()
    };

    let h = &ctx.headers;
    h.assign_str(X_AMZ_ACL, &mut input.acl);
    h.assign_str(CACHE_CONTROL, &mut input.cache_control);
    h.assign_str(CONTENT_DISPOSITION, &mut input.content_disposition);
    h.assign_str(CONTENT_ENCODING, &mut input.content_encoding);
    h.assign_str(CONTENT_LANGUAGE, &mut input.content_language);
    h.assign_str(CONTENT_TYPE, &mut input.content_type);
    h.assign_str(X_AMZ_COPY_SOURCE_IF_MATCH, &mut input.copy_source_if_match);
    h.assign_str(
        X_AMZ_COPY_SOURCE_IF_MODIFIED_SINCE,
        &mut input.copy_source_if_modified_since,
    );
    h.assign_str(
        X_AMZ_COPY_SOURCE_IF_NONE_MATCH,
        &mut input.copy_source_if_none_match,
    );
    h.assign_str(
        X_AMZ_COPY_SOURCE_IF_UNMODIFIED_SINCE,
        &mut input.copy_source_if_unmodified_since,
    );
    h.assign_str(EXPIRES, &mut input.expires);
    h.assign_str(X_AMZ_GRANT_FULL_CONTROL, &mut input.grant_full_control);
    h.assign_str(X_AMZ_GRANT_READ, &mut input.grant_read);
    h.assign_str(X_AMZ_GRANT_READ_ACP, &mut input.grant_read_acp);
    h.assign_str(X_AMZ_GRANT_WRITE_ACP, &mut input.grant_write_acp);
    h.assign_str(X_AMZ_METADATA_DIRECTIVE, &mut input.metadata_directive);
    h.assign_str(X_AMZ_TAGGING_DIRECTIVE, &mut input.tagging_directive);
    h.assign_str(
        X_AMZ_SERVER_SIDE_ENCRYPTION,
        &mut input.server_side_encryption,
    );
    h.assign_str(X_AMZ_STORAGE_CLASS, &mut input.storage_class);
    h.assign_str(
        X_AMZ_WEBSITE_REDIRECT_LOCATION,
        &mut input.website_redirect_location,
    );
    h.assign_str(
        X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
        &mut input.sse_customer_algorithm,
    );
    h.assign_str(
        X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
        &mut input.sse_customer_key,
    );
    h.assign_str(
        X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
        &mut input.sse_customer_key_md5,
    );
    h.assign_str(
        X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
        &mut input.ssekms_key_id,
    );
    h.assign_str(
        X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT,
        &mut input.ssekms_encryption_context,
    );
    h.assign_str(
        X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
        &mut input.copy_source_sse_customer_algorithm,
    );
    h.assign_str(
        X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
        &mut input.copy_source_sse_customer_key,
    );
    h.assign_str(
        X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
        &mut input.copy_source_sse_customer_key_md5,
    );
    h.assign_str(X_AMZ_REQUEST_PAYER, &mut input.request_payer);
    h.assign_str(X_AMZ_TAGGING, &mut input.tagging);
    h.assign_str(X_AMZ_OBJECT_LOCK_MODE, &mut input.object_lock_mode);
    h.assign_str(
        X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE,
        &mut input.object_lock_retain_until_date,
    );
    h.assign_str(
        X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
        &mut input.object_lock_legal_hold_status,
    );

    Ok(input)
}

impl S3Output for CopyObjectOutput {
    #[allow(clippy::shadow_unrelated)]
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_optional_header(X_AMZ_EXPIRATION, self.expiration)?;
            res.set_optional_header(X_AMZ_COPY_SOURCE_VERSION_ID, self.copy_source_version_id)?;
            res.set_optional_header(X_AMZ_VERSION_ID, self.version_id)?;
            res.set_optional_header(X_AMZ_SERVER_SIDE_ENCRYPTION, self.server_side_encryption)?;
            res.set_optional_header(
                X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
                self.sse_customer_algorithm,
            )?;
            res.set_optional_header(
                X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
                self.sse_customer_key_md5,
            )?;
            res.set_optional_header(
                X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
                self.ssekms_key_id,
            )?;
            res.set_optional_header(
                X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT,
                self.ssekms_encryption_context,
            )?;
            res.set_optional_header(X_AMZ_REQUEST_CHARGED, self.request_charged)?;

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

impl From<CopyObjectError> for S3Error {
    fn from(e: CopyObjectError) -> Self {
        match e {
            CopyObjectError::ObjectNotInActiveTierError(msg) => {
                Self::new(S3ErrorCode::ObjectNotInActiveTierError, msg)
            }
        }
    }
}
