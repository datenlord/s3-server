//! [`PutObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{PutObjectError, PutObjectOutput, PutObjectRequest};
use crate::errors::{S3Error, S3ErrorCode, S3Result};
use crate::headers::{
    CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_LENGTH,
    CONTENT_MD5, CONTENT_TYPE, ETAG, EXPIRES, X_AMZ_ACL, X_AMZ_EXPIRATION,
    X_AMZ_GRANT_FULL_CONTROL, X_AMZ_GRANT_READ, X_AMZ_GRANT_READ_ACP, X_AMZ_GRANT_WRITE_ACP,
    X_AMZ_OBJECT_LOCK_LEGAL_HOLD, X_AMZ_OBJECT_LOCK_MODE, X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE,
    X_AMZ_REQUEST_CHARGED, X_AMZ_REQUEST_PAYER, X_AMZ_SERVER_SIDE_ENCRYPTION,
    X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID, X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5, X_AMZ_STORAGE_CLASS, X_AMZ_TAGGING,
    X_AMZ_VERSION_ID, X_AMZ_WEBSITE_REDIRECT_LOCATION,
};
use crate::output::S3Output;
use crate::path::S3Path;
use crate::storage::S3Storage;
use crate::streams::multipart::Multipart;
use crate::utils::body::{transform_body_stream, transform_file_stream};
use crate::utils::{Apply, ResponseExt};
use crate::{async_trait, Method, Response};

use std::collections::HashMap;
use std::mem;

/// `PutObject` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        if ctx.req.method() == Method::POST {
            bool_try!(ctx.path.is_bucket());
            ctx.multipart.is_some()
        } else if ctx.req.method() == Method::PUT {
            bool_try!(ctx.path.is_object());
            ctx.query_strings.is_none()
        } else {
            false
        }
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.put_object(input).await;
        output.try_into_response()
    }
}

/// extract from multipart
fn extract_from_multipart(input: &mut PutObjectRequest, mut multipart: Multipart) -> S3Result<()> {
    multipart.assign_str("acl", &mut input.acl);
    multipart.assign_str("content-type", &mut input.content_type);
    multipart.assign_str("expires", &mut input.expires);
    multipart.assign_str("tagging", &mut input.tagging);
    multipart.assign_str("x-amz-storage-class", &mut input.storage_class);

    let mut metadata: HashMap<String, String> = HashMap::new();
    for &mut (ref mut name, ref mut value) in &mut multipart.fields {
        name.make_ascii_lowercase();
        let meta_prefix = "x-amz-meta-";
        if name.starts_with(meta_prefix) {
            let (_, meta_key) = name.split_at(meta_prefix.len());
            if !meta_key.is_empty() {
                let _prev = metadata.insert(meta_key.to_owned(), mem::take(value));
            }
        }
    }
    if !metadata.is_empty() {
        input.metadata = Some(metadata);
    }
    // TODO: how to handle the other fields?

    let file_stream = multipart.file.stream;

    input.body = file_stream.apply(transform_file_stream).apply(Some);

    Ok(())
}

/// extract operation request
fn extract(ctx: &mut ReqContext<'_>) -> S3Result<PutObjectRequest> {
    let (bucket, key) = if ctx.req.method() == Method::POST {
        let bucket = ctx.unwrap_bucket_path();

        #[allow(clippy::unwrap_used)]
        let multipart = ctx.multipart.as_ref().unwrap();

        let key = multipart
            .find_field_value("key")
            .ok_or_else(|| S3Error::new(S3ErrorCode::UserKeyMustBeSpecified, "Missing key"))?;

        if !S3Path::check_key(key) {
            return Err(S3Error::new(
                S3ErrorCode::KeyTooLongError,
                "Your key is too long.",
            ));
        }

        (bucket, key)
    } else if ctx.req.method() == Method::PUT {
        ctx.unwrap_object_path()
    } else {
        panic!("unexpected method");
    };

    let mut input: PutObjectRequest = PutObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        body: None,
        ..PutObjectRequest::default()
    };

    let h = &ctx.headers;
    h.assign(CONTENT_LENGTH, &mut input.content_length)
        .map_err(|err| invalid_request!("Invalid header: content-length", err))?;

    h.assign_str(&*X_AMZ_ACL, &mut input.acl);
    h.assign_str(CACHE_CONTROL, &mut input.cache_control);
    h.assign_str(CONTENT_DISPOSITION, &mut input.content_disposition);
    h.assign_str(CONTENT_ENCODING, &mut input.content_encoding);
    h.assign_str(CONTENT_LANGUAGE, &mut input.content_language);
    h.assign_str(&*CONTENT_MD5, &mut input.content_md5);
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

    let mut metadata: HashMap<String, String> = HashMap::new();
    for &(name, value) in ctx.headers.as_ref() {
        let meta_prefix = "x-amz-meta-";
        if name.starts_with(meta_prefix) {
            let (_, meta_key) = name.split_at(meta_prefix.len());
            if !meta_key.is_empty() {
                let _prev = metadata.insert(meta_key.to_owned(), value.to_owned());
            }
        }
    }
    if !metadata.is_empty() {
        input.metadata = Some(metadata);
    }

    match ctx.multipart.take() {
        None => input.body = ctx.take_body().apply(transform_body_stream).apply(Some),
        Some(multipart) => extract_from_multipart(&mut input, multipart)?,
    };

    Ok(input)
}

impl S3Output for PutObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_optional_header(&*X_AMZ_EXPIRATION, self.expiration)?;
            res.set_optional_header(ETAG, self.e_tag)?;
            res.set_optional_header(&*X_AMZ_SERVER_SIDE_ENCRYPTION, self.server_side_encryption)?;
            res.set_optional_header(&*X_AMZ_VERSION_ID, self.version_id)?;
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
            Ok(())
        })
    }
}

impl From<PutObjectError> for S3Error {
    fn from(e: PutObjectError) -> Self {
        match e {}
    }
}
