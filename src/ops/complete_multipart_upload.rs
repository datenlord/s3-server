//! [`CompleteMultipartUpload`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CompleteMultipartUpload.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{
    CompleteMultipartUploadError, CompleteMultipartUploadOutput, CompleteMultipartUploadRequest,
    CompletedMultipartUpload, CompletedPart,
};
use crate::errors::{S3Error, S3Result};
use crate::headers::{
    X_AMZ_EXPIRATION, X_AMZ_REQUEST_CHARGED, X_AMZ_REQUEST_PAYER, X_AMZ_SERVER_SIDE_ENCRYPTION,
    X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID, X_AMZ_VERSION_ID,
};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::body::deserialize_xml_body;
use crate::utils::{ResponseExt, XmlWriterExt};
use crate::{async_trait, Response};

use hyper::Method;

/// `CompleteMultipartUpload` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::POST);
        bool_try!(ctx.path.is_object());
        let qs = bool_try_some!(ctx.query_strings.as_ref());
        qs.get("uploadId").is_some()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx).await?;
        let output = storage.complete_multipart_upload(input).await;
        output.try_into_response()
    }
}

/// extract operation request
async fn extract(ctx: &mut ReqContext<'_>) -> S3Result<CompleteMultipartUploadRequest> {
    let multipart_upload: Option<self::xml::CompletedMultipartUpload> =
        deserialize_xml_body(ctx.take_body())
            .await
            .map_err(|err| invalid_request!("Invalid xml format", err))?;

    let (bucket, key) = ctx.unwrap_object_path();
    let upload_id = ctx.unwrap_qs("uploadId").to_owned();

    let mut input = CompleteMultipartUploadRequest {
        bucket: bucket.into(),
        key: key.into(),
        upload_id,
        multipart_upload: multipart_upload.map(Into::into),
        ..CompleteMultipartUploadRequest::default()
    };

    let h = &ctx.headers;
    h.assign_str(X_AMZ_REQUEST_PAYER, &mut input.request_payer);

    Ok(input)
}

impl From<CompleteMultipartUploadError> for S3Error {
    fn from(err: CompleteMultipartUploadError) -> Self {
        match err {}
    }
}

impl S3Output for CompleteMultipartUploadOutput {
    #[allow(clippy::shadow_unrelated)]
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_optional_header(X_AMZ_EXPIRATION, self.expiration)?;
            res.set_optional_header(X_AMZ_SERVER_SIDE_ENCRYPTION, self.server_side_encryption)?;
            res.set_optional_header(X_AMZ_VERSION_ID, self.version_id)?;
            res.set_optional_header(
                X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
                self.ssekms_key_id,
            )?;
            res.set_optional_header(X_AMZ_REQUEST_CHARGED, self.request_charged)?;

            let location = self.location;
            let bucket = self.bucket;
            let key = self.key;
            let e_tag = self.e_tag;

            res.set_xml_body(256, |w| {
                w.stack("CompleteMultipartUploadResult", |w| {
                    w.opt_element("Location", location)?;
                    w.opt_element("Bucket", bucket)?;
                    w.opt_element("Key", key)?;
                    w.opt_element("ETag", e_tag)?;
                    Ok(())
                })
            })?;

            Ok(())
        })
    }
}

mod xml {
    //! xml repr

    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    /// `CompletedMultipartUpload`
    pub struct CompletedMultipartUpload {
        /// Part
        #[serde(rename = "Part")]
        parts: Option<Vec<CompletedPart>>,
    }

    /// `CompletedPart`
    #[derive(Debug, Deserialize)]
    pub struct CompletedPart {
        /// ETag
        #[serde(rename = "ETag")]
        e_tag: Option<String>,
        /// PartNumber
        #[serde(rename = "PartNumber")]
        part_number: Option<i64>,
    }

    impl From<CompletedMultipartUpload> for super::CompletedMultipartUpload {
        fn from(m: CompletedMultipartUpload) -> Self {
            Self {
                parts: m.parts.map(|v| v.into_iter().map(From::from).collect()),
            }
        }
    }

    impl From<CompletedPart> for super::CompletedPart {
        fn from(p: CompletedPart) -> Self {
            Self {
                e_tag: p.e_tag,
                part_number: p.part_number,
            }
        }
    }
}
