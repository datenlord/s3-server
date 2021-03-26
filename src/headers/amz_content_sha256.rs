//! x-amz-content-sha256

use crate::utils::{crypto, Apply};

/// `x-amz-content-sha256`
///
/// See [Common Request Headers](https://docs.aws.amazon.com/AmazonS3/latest/API/RESTCommonRequestHeaders.html)
#[derive(Debug)]
#[allow(clippy::exhaustive_enums)]
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

impl<'a> AmzContentSha256<'a> {
    /// parse `ContentSha256` from `x-amz-content-sha256` header
    /// # Errors
    /// Returns an `Err` if the header is invalid
    pub fn from_header_str(header: &'a str) -> Result<Self, ParseAmzContentSha256Error> {
        match header {
            "UNSIGNED-PAYLOAD" => Self::UnsignedPayload,
            "STREAMING-AWS4-HMAC-SHA256-PAYLOAD" => Self::MultipleChunks,
            payload_checksum => {
                if !crypto::is_sha256_checksum(payload_checksum) {
                    return Err(ParseAmzContentSha256Error { _priv: () });
                }
                Self::SingleChunk { payload_checksum }
            }
        }
        .apply(Ok)
    }
}
