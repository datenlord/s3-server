//! [`UploadPart`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_UploadPart.html)

use crate::{
    dto::{ByteStream, UploadPartError, UploadPartOutput, UploadPartRequest},
    headers::names::{
        CONTENT_MD5, X_AMZ_REQUEST_CHARGED, X_AMZ_SERVER_SIDE_ENCRYPTION,
        X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
        X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
        X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
    },
    output::wrap_output,
    utils::Apply,
    utils::RequestExt,
    utils::ResponseExt,
    Body, BoxStdError, Request, Response, S3Output, S3Result,
};

use std::io;

use futures::StreamExt;
use hyper::header::{CONTENT_LENGTH, ETAG};

impl S3Output for UploadPartError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}

impl S3Output for UploadPartOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_optional_header(|| ETAG, self.e_tag)?;

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

            res.set_optional_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;

            Ok(())
        })
    }
}

/// transform stream
fn transform_stream(body: Body) -> ByteStream {
    body.map(|try_chunk| {
        try_chunk.map(|c| c).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Error obtaining chunk: {}", e),
            )
        })
    })
    .apply(ByteStream::new)
}

/// extract operation request
pub fn extract(
    req: &Request,
    bucket: &str,
    key: &str,
    part_number: i64,
    upload_id: String,
    body: Body,
) -> Result<UploadPartRequest, BoxStdError> {
    let body = transform_stream(body);

    let mut input = UploadPartRequest {
        bucket: bucket.into(),
        key: key.into(),
        part_number,
        upload_id,
        body: Some(body),
        ..UploadPartRequest::default()
    };

    req.assign_from_optional_header(CONTENT_LENGTH, &mut input.content_length)?;
    req.assign_from_optional_header(&*CONTENT_MD5, &mut input.content_md5)?;
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

    Ok(input)
}
