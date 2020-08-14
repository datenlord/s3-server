//! A location in the S3 storage.
//!
//! + [Request styles](https://docs.aws.amazon.com/AmazonS3/latest/dev/RESTAPI.html#virtual-hosted-path-style-requests)
//! + [Bucket nameing rules](https://docs.aws.amazon.com/AmazonS3/latest/dev/BucketRestrictions.html#bucketnamingrules)

use std::net::IpAddr;

#[derive(Debug)]
pub enum S3Path<'a> {
    Root,
    Bucket { bucket: &'a str },
    Object { bucket: &'a str, key: &'a str },
}

#[derive(Debug, thiserror::Error)]
#[error("ParseS3PathError: {:?}",.kind)]
pub struct ParseS3PathError {
    kind: S3PathErrorKind,
}

impl ParseS3PathError {
    #[must_use]
    pub const fn kind(&self) -> &S3PathErrorKind {
        &self.kind
    }
}

#[allow(missing_copy_implementations)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S3PathErrorKind {
    InvalidPath,
    InvalidBucketName,
    TooLongKey,
}

macro_rules! short_circuit_check{
    {$($cond:expr;)+}=>{{
        $({
            let cond = $cond;
            if !cond {
                return false;
            }

        })+
        true
    }}
}

fn check_bucket_name(name: &str) -> bool {
    fn is_digit_or_lowercase(b: u8) -> bool {
        b.is_ascii_lowercase() || b.is_ascii_digit()
    }

    short_circuit_check! {
        (3_usize..64).contains(&name.len());
        name.as_bytes().iter().all(|&b|is_digit_or_lowercase(b) || b==b'.' || b==b'-');
        name.as_bytes().first().map(|&b|is_digit_or_lowercase(b))==Some(true);
        name.as_bytes().last().map(|&b|is_digit_or_lowercase(b))==Some(true);
        name.parse::<IpAddr>().is_err();
        !name.starts_with("xn--");
    }
}

const fn check_key(key: &str) -> bool {
    key.len() <= 1024
}

impl<'a> S3Path<'a> {
    pub fn try_from_path(path: &'a str) -> Result<Self, ParseS3PathError> {
        if !path.starts_with('/') {
            return Err(ParseS3PathError {
                kind: S3PathErrorKind::InvalidPath,
            });
        }

        let mut iter = path.split('/');
        let _ = iter.next().ok_or_else(|| ParseS3PathError {
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

        if !check_bucket_name(bucket) {
            return Err(ParseS3PathError {
                kind: S3PathErrorKind::InvalidBucketName,
            });
        }

        let key = match iter.next() {
            None | Some("") => return Ok(S3Path::Bucket { bucket }),

            // here can not panic, because `split` ensures `path` has enough length
            Some(_) => path.get(bucket.len().saturating_add(2)..).unwrap(),
        };

        if !check_key(key) {
            return Err(ParseS3PathError {
                kind: S3PathErrorKind::TooLongKey,
            });
        }

        Ok(Self::Object { bucket, key })
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
            &S3PathErrorKind::TooLongKey
        );
    }
}
