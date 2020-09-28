//! [`CompleteMultipartUpload`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CompleteMultipartUpload.html)

use crate::{
    dto::{
        CompleteMultipartUploadError, CompleteMultipartUploadOutput,
        CompleteMultipartUploadRequest, CompletedMultipartUpload, CompletedPart,
    },
    headers::names::{
        X_AMZ_EXPIRATION, X_AMZ_REQUEST_CHARGED, X_AMZ_REQUEST_PAYER, X_AMZ_SERVER_SIDE_ENCRYPTION,
        X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID, X_AMZ_VERSION_ID,
    },
    output::wrap_output,
    utils::{deserialize_xml_body, RequestExt, ResponseExt, XmlWriterExt},
    Body, BoxStdError, Request, Response, S3Output, S3Result,
};

impl S3Output for CompleteMultipartUploadError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}

impl S3Output for CompleteMultipartUploadOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_optional_header(|| X_AMZ_EXPIRATION.clone(), self.expiration)?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION.clone(),
                self.server_side_encryption,
            )?;
            res.set_optional_header(|| X_AMZ_VERSION_ID.clone(), self.version_id)?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID.clone(),
                self.ssekms_key_id,
            )?;
            res.set_optional_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;

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

/// extract operation request
pub async fn extract(
    req: &Request,
    body: Body,
    bucket: &str,
    key: &str,
    upload_id: String,
) -> Result<CompleteMultipartUploadRequest, BoxStdError> {
    let multipart_upload: Option<self::xml::CompletedMultipartUpload> =
        deserialize_xml_body(body).await?;

    let mut input = CompleteMultipartUploadRequest {
        bucket: bucket.into(),
        key: key.into(),
        upload_id,
        multipart_upload: multipart_upload.map(Into::into),
        ..CompleteMultipartUploadRequest::default()
    };

    req.assign_from_optional_header(&*X_AMZ_REQUEST_PAYER, &mut input.request_payer)?;

    Ok(input)
}
