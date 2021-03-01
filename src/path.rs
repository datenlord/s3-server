//! A path in the S3 storage.
//!
//! + [Request styles](https://docs.aws.amazon.com/AmazonS3/latest/dev/RESTAPI.html#virtual-hosted-path-style-requests)
//! + [Bucket nameing rules](https://docs.aws.amazon.com/AmazonS3/latest/dev/BucketRestrictions.html#bucketnamingrules)

use std::net::IpAddr;

/// A path in the S3 storage
#[derive(Debug)]
pub enum S3Path<'a> {
    /// Root path
    Root,
    /// Bucket path
    Bucket {
        /// Bucket name
        bucket: &'a str,
    },
    /// Object path
    Object {
        /// Bucket name
        bucket: &'a str,
        /// Object key
        key: &'a str,
    },
}

// Why allow `missing_copy_implementations` ?
// 1. We can't yet guarantee that the error type is `Copy` in the future.
// 2. A copyable error type is strange. `std::num::ParseIntError` is `Clone` but not `Copy`.

/// An error which can be returned when parsing a s3 path
#[allow(missing_copy_implementations)]
#[derive(Debug, thiserror::Error)]
#[error("ParseS3PathError: {:?}",.kind)]
pub struct ParseS3PathError {
    /// error kind
    kind: S3PathErrorKind,
}

impl ParseS3PathError {
    /// Returns the corresponding `S3PathErrorKind` for this error
    #[must_use]
    pub const fn kind(&self) -> &S3PathErrorKind {
        &self.kind
    }
}

/// A list of `ParseS3PathError` reasons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S3PathErrorKind {
    /// The request is not a valid path-style request
    InvalidPath,
    /// The bucket name is invalid
    InvalidBucketName,
    /// The object key is too long
    KeyTooLong,
}

impl<'a> S3Path<'a> {
    /// See [bucket nameing rules](https://docs.aws.amazon.com/AmazonS3/latest/dev/BucketRestrictions.html#bucketnamingrules)
    #[must_use]
    pub fn check_bucket_name(name: &str) -> bool {
        if !(3_usize..64).contains(&name.len()) {
            return false;
        }

        if !name
            .as_bytes()
            .iter()
            .all(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        {
            return false;
        }

        if name
            .as_bytes()
            .first()
            .map(|&b| b.is_ascii_lowercase() || b.is_ascii_digit())
            != Some(true)
        {
            return false;
        }

        if name
            .as_bytes()
            .last()
            .map(|&b| b.is_ascii_lowercase() || b.is_ascii_digit())
            != Some(true)
        {
            return false;
        }

        if name.parse::<IpAddr>().is_ok() {
            return false;
        }

        if name.starts_with("xn--") {
            return false;
        }

        true
    }

    /// The name for a key is a sequence of Unicode characters whose UTF-8 encoding is at most 1,024 bytes long.
    /// See [object keys](https://docs.aws.amazon.com/AmazonS3/latest/dev/UsingMetadata.html#object-keys)
    #[must_use]
    pub const fn check_key(key: &str) -> bool {
        key.len() <= 1024
    }

    /// Parse a path-style request
    /// # Errors
    /// Returns an `Err` if the s3 path is invalid
    pub fn try_from_path(path: &'a str) -> Result<Self, ParseS3PathError> {
        if !path.starts_with('/') {
            return Err(ParseS3PathError {
                kind: S3PathErrorKind::InvalidPath,
            });
        }

        let mut iter = path.split('/');
        let _ = iter.next().ok_or(ParseS3PathError {
            kind: S3PathErrorKind::InvalidPath,
        })?;

        let bucket = match iter.next() {
            None => {
                return Err(ParseS3PathError {
                    kind: S3PathErrorKind::InvalidPath,
                })
            }
            Some("") => return Ok(S3Path::Root),
            Some(s) => s,
        };

        if !Self::check_bucket_name(bucket) {
            return Err(ParseS3PathError {
                kind: S3PathErrorKind::InvalidBucketName,
            });
        }

        let key = match iter.next() {
            None | Some("") => return Ok(S3Path::Bucket { bucket }),

            // here can not panic, because `split` ensures `path` has enough length
            #[allow(clippy::indexing_slicing)]
            Some(_) => &path[bucket.len().saturating_add(2)..],
        };

        if !Self::check_key(key) {
            return Err(ParseS3PathError {
                kind: S3PathErrorKind::KeyTooLong,
            });
        }

        Ok(Self::Object { bucket, key })
    }

    /// is root
    #[must_use]
    pub const fn is_root(&self) -> bool {
        matches!(*self, Self::Root)
    }

    /// is bucket
    #[must_use]
    pub const fn is_bucket(&self) -> bool {
        matches!(*self, Self::Bucket { .. })
    }

    /// is object
    #[must_use]
    pub const fn is_object(&self) -> bool {
        matches!(*self, Self::Object { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter;

    #[test]
    fn parse_s3_path() {
        assert!(matches!(S3Path::try_from_path("/"), Ok(S3Path::Root)));

        assert!(matches!(
            S3Path::try_from_path("/bucket"),
            Ok(S3Path::Bucket { bucket: "bucket" })
        ));

        assert!(matches!(
            S3Path::try_from_path("/bucket/"),
            Ok(S3Path::Bucket { bucket: "bucket" })
        ));

        assert!(matches!(
            S3Path::try_from_path("/bucket/dir/object"),
            Ok(S3Path::Object {
                bucket: "bucket",
                key: "dir/object"
            })
        ));

        assert_eq!(
            S3Path::try_from_path("asd").unwrap_err().kind(),
            &S3PathErrorKind::InvalidPath
        );

        assert_eq!(
            S3Path::try_from_path("a/").unwrap_err().kind(),
            &S3PathErrorKind::InvalidPath
        );

        assert_eq!(
            S3Path::try_from_path("/*").unwrap_err().kind(),
            &S3PathErrorKind::InvalidBucketName
        );

        let too_long_path = format!(
            "/{}/{}",
            "asd",
            iter::repeat('b').take(2048).collect::<String>().as_str()
        );

        assert_eq!(
            S3Path::try_from_path(&too_long_path).unwrap_err().kind(),
            &S3PathErrorKind::KeyTooLong
        );
    }
}
