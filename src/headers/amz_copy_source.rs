//! x-amz-copy-source

use crate::path::S3Path;

use regex::Regex;

/// x-amz-copy-source
#[derive(Debug)]
pub enum AmzCopySource<'a> {
    /// bucket repr
    Bucket {
        /// bucket
        bucket: &'a str,
        /// key
        key: &'a str,
    },
    /// access point repr
    AccessPoint {
        /// region
        region: &'a str,
        /// account id
        account_id: &'a str,
        /// access point name
        access_point_name: &'a str,
        /// key
        key: &'a str,
    },
}

/// `ParseAmzCopySourceError`
#[allow(missing_copy_implementations)] // Why? See `crate::path::ParseS3PathError`.
#[derive(Debug, thiserror::Error)]
pub enum ParseAmzCopySourceError {
    /// pattern mismatch
    #[error("ParseAmzCopySourceError: PatternMismatch")]
    PatternMismatch,

    /// invalid bucket name
    #[error("ParseAmzCopySourceError: InvalidBucketName")]
    InvalidBucketName,

    /// invalid key
    #[error("ParseAmzCopySourceError: InvalidKey")]
    InvalidKey,
}

impl<'a> AmzCopySource<'a> {
    /// Checks header pattern
    /// # Errors
    /// Returns an error if the header does not match the pattern
    pub fn try_match(header: &str) -> Result<(), ParseAmzCopySourceError> {
        // x-amz-copy-source header pattern
        let pattern: &Regex = static_regex!(".+?/.+");

        if pattern.is_match(header) {
            Ok(())
        } else {
            Err(ParseAmzCopySourceError::PatternMismatch)
        }
    }

    /// Parses `AmzCopySource` from header
    /// # Errors
    /// Returns an error if the header is invalid
    #[allow(clippy::clippy::unwrap_in_result)]
    pub fn from_header_str(header: &'a str) -> Result<Self, ParseAmzCopySourceError> {
        // TODO: support access point
        // TODO: use nom parser

        // bucket pattern
        let pattern: &Regex = static_regex!("^(.+?)/(.+)$");

        match pattern.captures(header) {
            None => Err(ParseAmzCopySourceError::PatternMismatch),
            Some(captures) => {
                let bucket = captures.get(1).unwrap().as_str();
                let key = captures.get(2).unwrap().as_str();

                if !S3Path::check_bucket_name(bucket) {
                    return Err(ParseAmzCopySourceError::InvalidBucketName);
                }

                if !S3Path::check_key(key) {
                    return Err(ParseAmzCopySourceError::InvalidKey);
                }

                Ok(Self::Bucket { bucket, key })
            }
        }
    }
}
