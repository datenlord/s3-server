//! [`CreateMultipartUpload`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateMultipartUpload.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{
    CreateMultipartUploadError, CreateMultipartUploadOutput, CreateMultipartUploadRequest,
};
use crate::errors::{S3Error, S3Result};
use crate::headers::{
    CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_TYPE, EXPIRES,
    X_AMZ_ABORT_DATE, X_AMZ_ABORT_RULE_ID, X_AMZ_ACL, X_AMZ_GRANT_FULL_CONTROL, X_AMZ_GRANT_READ,
    X_AMZ_GRANT_READ_ACP, X_AMZ_GRANT_WRITE_ACP, X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
    X_AMZ_OBJECT_LOCK_MODE, X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE, X_AMZ_REQUEST_CHARGED,
    X_AMZ_REQUEST_PAYER, X_AMZ_SERVER_SIDE_ENCRYPTION, X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
    X_AMZ_STORAGE_CLASS, X_AMZ_TAGGING, X_AMZ_WEBSITE_REDIRECT_LOCATION,
};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::ResponseExt;
use crate::utils::XmlWriterExt;
use crate::{async_trait, Method, Response};

/// `CreateMultipartUpload` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::POST);
        bool_try!(ctx.path.is_object());
        let qs = bool_try_some!(ctx.query_strings.as_ref());
        qs.get("uploads").is_some()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.create_multipart_upload(input).await;
        output.try_into_response()
    }
}

impl From<CreateMultipartUploadError> for S3Error {
    fn from(e: CreateMultipartUploadError) -> Self {
        match e {}
    }
}

impl S3Output for CreateMultipartUploadOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_optional_header(&*X_AMZ_ABORT_DATE, self.abort_date)?;
            res.set_optional_header(&*X_AMZ_ABORT_RULE_ID, self.abort_rule_id)?;
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
            res.set_optional_header(
                &*X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT,
                self.ssekms_encryption_context,
            )?;

            res.set_optional_header(&*X_AMZ_REQUEST_CHARGED, self.request_charged)?;

            let bucket = self.bucket;
            let key = self.key;
            let upload_id = self.upload_id;

            res.set_xml_body(256, |w| {
                w.stack("InitialMultipartUploadResult", |w| {
                    w.opt_element("Bucket", bucket)?;
                    w.opt_element("Key", key)?;
                    w.opt_element("UploadId", upload_id)?;
                    Ok(())
                })
            })?;

            Ok(())
        })
    }
}

/// extract operation request
fn extract(ctx: &mut ReqContext<'_>) -> S3Result<CreateMultipartUploadRequest> {
    let (bucket, key) = ctx.unwrap_object_path();

    let mut input = CreateMultipartUploadRequest {
        bucket: bucket.into(),
        key: key.into(),
        ..CreateMultipartUploadRequest::default()
    };

    let h = &ctx.headers;
    h.assign_str(&*X_AMZ_ACL, &mut input.acl);
    h.assign_str(CACHE_CONTROL, &mut input.cache_control);
    h.assign_str(CONTENT_DISPOSITION, &mut input.content_disposition);
    h.assign_str(CONTENT_ENCODING, &mut input.content_encoding);
    h.assign_str(CONTENT_LANGUAGE, &mut input.content_language);
    h.assign_str(CONTENT_TYPE, &mut input.content_type);
    h.assign_str(EXPIRES, &mut input.expires);
    h.assign_str(&*X_AMZ_GRANT_FULL_CONTROL, &mut input.grant_full_control);
    h.assign_str(&*X_AMZ_GRANT_READ, &mut input.grant_read);
    h.assign_str(&*X_AMZ_GRANT_READ_ACP, &mut input.grant_read_acp);
    h.assign_str(&*X_AMZ_GRANT_WRITE_ACP, &mut input.grant_write_acp);
    h.assign_str(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION,
        &mut input.server_side_encryption,
    );
    h.assign_str(&*X_AMZ_STORAGE_CLASS, &mut input.storage_class);
    h.assign_str(
        &*X_AMZ_WEBSITE_REDIRECT_LOCATION,
        &mut input.website_redirect_location,
    );
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
    h.assign_str(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
        &mut input.ssekms_key_id,
    );
    h.assign_str(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT,
        &mut input.ssekms_encryption_context,
    );
    h.assign_str(&*X_AMZ_REQUEST_PAYER, &mut input.request_payer);
    h.assign_str(&*X_AMZ_TAGGING, &mut input.tagging);
    h.assign_str(&*X_AMZ_OBJECT_LOCK_MODE, &mut input.object_lock_mode);
    h.assign_str(
        &*X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE,
        &mut input.object_lock_retain_until_date,
    );
    h.assign_str(
        &*X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
        &mut input.object_lock_legal_hold_status,
    );

    Ok(input)
}
