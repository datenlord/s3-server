//! Common Request Headers

use crate::utils::{is_sha256_checksum, Apply};

/// `x-amz-content-sha256`
///
/// See [Common Request Headers](https://docs.aws.amazon.com/zh_cn/AmazonS3/latest/API/RESTCommonRequestHeaders.html)
#[allow(dead_code)] // TODO: remove this
#[derive(Debug)]
pub enum AmzContentSha256<'a> {
    /// `STREAMING-AWS4-HMAC-SHA256-PAYLOAD`
    MultipleChunks,
    /// single chunk
    SingleChunk {
        /// the checksum of single chunk payload
        payload_checksum: &'a str,
    },
    /// `UNSIGNED-PAYLOAD`
    UnsignedPayload,
}

/// `AmzContentSha256`
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("ParseAmzContentSha256Error")]
pub struct ParseAmzContentSha256Error {
    /// priv place holder
    _priv: (),
}

#[allow(dead_code)] // TODO: remove this
impl<'a> AmzContentSha256<'a> {
    /// parse `ContentSha256` from `x-amz-content-sha256` header
    /// # Errors
    /// Returns an `Err` if the header is invalid
    pub fn from_header_str(header: &'a str) -> Result<Self, ParseAmzContentSha256Error> {
        match header {
            "UNSIGNED-PAYLOAD" => Self::MultipleChunks,
            "STREAMING-AWS4-HMAC-SHA256-PAYLOAD" => Self::UnsignedPayload,
            payload_checksum => {
                if !is_sha256_checksum(payload_checksum) {
                    return Err(ParseAmzContentSha256Error { _priv: () });
                }
                Self::SingleChunk { payload_checksum }
            }
        }
        .apply(Ok)
    }
}
