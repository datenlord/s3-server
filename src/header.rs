#![allow(dead_code)] // TODO: remove this
#![allow(missing_copy_implementations)]

//! Common Request Headers

use crate::utils::{is_sha256_checksum, Apply};

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// `x-amz-content-sha256`
///
/// See [Common Request Headers](https://docs.aws.amazon.com/zh_cn/AmazonS3/latest/API/RESTCommonRequestHeaders.html)
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

/// See [sigv4-auth-using-authorization-header](https://docs.aws.amazon.com/zh_cn/AmazonS3/latest/API/sigv4-auth-using-authorization-header.html)
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct AuthorizationV4<'a> {
    /// The algorithm that was used to calculate the signature.
    algorithm: &'a str,

    /// Access key ID and the scope information, which includes the date, Region, and service that were used to calculate the signature.
    credential: CredentialV4<'a>,

    /// A semicolon-separated list of request headers that you used to compute `Signature`.
    signed_headers: Vec<&'a str>,

    /// The 256-bit signature expressed as 64 lowercase hexadecimal characters.
    signature: &'a str,
}

/// Access key ID and the scope information, which includes the date, Region, and service that were used to calculate the signature.
///
/// This string has the following form:
/// `<your-access-key-id>/<date>/<aws-region>/<aws-service>/aws4_request`
///
/// See [sigv4-auth-using-authorization-header](https://docs.aws.amazon.com/zh_cn/AmazonS3/latest/API/sigv4-auth-using-authorization-header.html)
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CredentialV4<'a> {
    /// access key id
    access_key_id: &'a str,
    /// <date> value is specified using YYYYMMDD format.
    date: &'a str,
    /// region
    aws_region: &'a str,
    /// <aws-service> value is `s3` when sending request to Amazon S3.
    aws_service: &'a str,
}

/// `ParseAuthorizationError`
#[derive(Debug, thiserror::Error)]
#[error("ParseAuthorizationError")]
pub(super) struct ParseAuthorizationError {
    /// priv place holder
    _priv: (),
}

impl<'a> AuthorizationV4<'a> {
    /// parse `AuthorizationV4` from `Authorization` header
    pub(super) fn from_header_str(
        auth: &'a str,
    ) -> Result<AuthorizationV4<'a>, ParseAuthorizationError> {
        #[allow(clippy::shadow_reuse)]
        /// nom parser
        fn parse(input: &str) -> nom::IResult<&str, AuthorizationV4<'_>> {
            use chrono::{TimeZone, Utc};
            use nom::{
                bytes::complete::{tag, take, take_till, take_till1},
                character::complete::{multispace0, multispace1},
                combinator::{all_consuming, verify},
                sequence::{terminated, tuple},
            };

            let slash_tail = terminated(take_till1(|c| c == '/'), tag("/"));
            let space_till1 = take_till1(|c: char| c.is_ascii_whitespace());
            let space_till0 = take_till(|c: char| c.is_ascii_whitespace());

            let (input, algorithm) = space_till1(input)?;

            let (input, _) = multispace1(input)?;

            let (input, _) = tag("Credential=")(input)?;
            let (input, access_key_id) = slash_tail(input)?;
            let (input, date) = slash_tail(input)?;

            let _ = verify(
                all_consuming(tuple((take(4_usize), take(2_usize), take(2_usize)))),
                |&(y, m, d): &(&str, &str, &str)| {
                    macro_rules! parse_num {
                        ($x:expr) => {{
                            match $x.parse() {
                                Ok(x) => x,
                                Err(_) => return false,
                            }
                        }};
                    }
                    matches!(
                        Utc.ymd_opt(parse_num!(y), parse_num!(m), parse_num!(d)),
                        chrono::LocalResult::Single(_)
                    )
                },
            )(date)?;

            let (input, aws_region) = slash_tail(input)?;
            let (input, aws_service) = slash_tail(input)?;
            let (input, _) = tag("aws4_request,")(input)?;

            let (input, _) = multispace1(input)?;

            let (mut input, _) = tag("SignedHeaders=")(input)?;
            let mut headers: SmallVec<[&str; 16]> = SmallVec::new();
            loop {
                let (remain, (header, sep)) =
                    tuple((take_till1(|c| c == ';' || c == ','), take(1_usize)))(input)?;

                input = remain;
                headers.push(header);
                if sep == "," {
                    break;
                }
            }

            let (input, _) = multispace1(input)?;

            let (input, _) = tag("Signature=")(input)?;
            let (input, signature) = space_till0(input)?;
            let (input, _) = all_consuming(multispace0)(input)?;

            let ans = AuthorizationV4 {
                algorithm,
                credential: CredentialV4 {
                    access_key_id,
                    date,
                    aws_region,
                    aws_service,
                },
                signed_headers: headers.into_vec(),
                signature,
            };

            Ok((input, ans))
        }

        match parse(auth) {
            Ok((_, ans)) => Ok(ans),
            Err(_) => Err(ParseAuthorizationError { _priv: () }),
        }
    }
}

/// `ParseCopySourceError`
#[derive(Debug, thiserror::Error)]
#[error("ParseCopySourceError")]
pub struct ParseCopySourceError {
    /// priv place holder
    _priv: (),
}

/// x-amz-copy-source
#[derive(Debug)]
pub enum CopySource<'a> {
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

#[allow(clippy::unwrap_used)] // for regex
impl<'a> CopySource<'a> {
    /// check header pattern
    /// # Errors
    /// Returns an error if the header does not match the pattern
    pub fn try_match(header: &str) -> Result<(), ParseCopySourceError> {
        /// x-amz-copy-source header pattern
        static PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(".+?/.+").unwrap());
        let pattern: &Regex = &*PATTERN;
        if pattern.is_match(header) {
            Ok(())
        } else {
            Err(ParseCopySourceError { _priv: () })
        }
    }

    /// parse `CopySource` from header
    /// # Errors
    /// Returns an error if the header is invalid
    pub fn from_header_str(header: &'a str) -> Result<Self, ParseCopySourceError> {
        /// bucket pattern
        static PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new("^(.+?)/(.+)$").unwrap());

        // TODO: support access point
        // TODO: use nom parser

        let pattern: &Regex = &*PATTERN;
        match pattern.captures(header) {
            None => Err(ParseCopySourceError { _priv: () }),
            Some(captures) => {
                let bucket = captures.get(1).unwrap().as_str();
                let key = captures.get(2).unwrap().as_str();
                if crate::path::check_bucket_name(bucket) && crate::path::check_key(key) {
                    Ok(Self::Bucket { bucket, key })
                } else {
                    Err(ParseCopySourceError { _priv: () })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_header() {
        {
            let auth = r#"AWS4-HMAC-SHA256 
                Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, 
                SignedHeaders=host;range;x-amz-date,
                Signature=fe5f80f77d5fa3beca038a248ff027d0445342fe2855ddc963176630326f1024
            "#;
            let ans = AuthorizationV4::from_header_str(auth).unwrap();

            assert_eq!(ans.algorithm, "AWS4-HMAC-SHA256");
            assert_eq!(ans.credential.access_key_id, "AKIAIOSFODNN7EXAMPLE");
            assert_eq!(ans.credential.date, "20130524");
            assert_eq!(ans.credential.aws_region, "us-east-1");
            assert_eq!(ans.credential.aws_service, "s3");
            assert_eq!(ans.signed_headers, &["host", "range", "x-amz-date"]);
            assert_eq!(
                ans.signature,
                "fe5f80f77d5fa3beca038a248ff027d0445342fe2855ddc963176630326f1024"
            );
        }
        {
            let auth = r#"AWS4-HMAC-SHA256 
                Credential=AKIAIOSFODNN7EXAMPLE/20200931/us-east-1/s3/aws4_request, 
                SignedHeaders=host;range;x-amz-date,
                Signature=fe5f80f77d5fa3beca038a248ff027d0445342fe2855ddc963176630326f1024
            "#;

            assert!(matches!(AuthorizationV4::from_header_str(auth), Err(_)));
        }
    }
}

pub mod names {
    //! Amz header names

    // TODO: declare const headers, see <https://github.com/hyperium/http/issues/264>

    use hyper::header::HeaderName;
    use once_cell::sync::Lazy;

    macro_rules! declare_header_name{
        [$($(#[$docs:meta])* $n:ident: $s:expr,)+] => {
            $(
                $(#[$docs])*
                pub static $n: Lazy<HeaderName> = Lazy::new(||HeaderName::from_static($s));
            )+

            #[cfg(test)]
            #[test]
            fn check_headers(){
                $(
                    dbg!(&*$n);
                )+
            }
        }
    }

    declare_header_name![
        /// x-amz-mfa
        X_AMZ_MFA: "x-amz-mfa",

        /// x-amz-content-sha256
        X_AMZ_CONTENT_SHA_256: "x-amz-content-sha256",

        /// x-amz-expiration
        X_AMZ_EXPIRATION: "x-amz-expiration",

        /// x-amz-copy-source-version-id
        X_AMZ_COPY_SOURCE_VERSION_ID: "x-amz-copy-source-version-id",

        /// x-amz-version-id
        X_AMZ_VERSION_ID: "x-amz-version-id",

        /// x-amz-request-charged
        X_AMZ_REQUEST_CHARGED: "x-amz-request-charged",

        /// x-amz-acl
        X_AMZ_ACL: "x-amz-acl",

        /// x-amz-copy-source
        X_AMZ_COPY_SOURCE: "x-amz-copy-source",

        /// x-amz-copy-source-if-match
        X_AMZ_COPY_SOURCE_IF_MATCH: "x-amz-copy-source-if-match",

        /// x-amz-copy-source-if-modified-since
        X_AMZ_COPY_SOURCE_IF_MODIFIED_SINCE: "x-amz-copy-source-if-modified-since",

        /// x-amz-copy-source-if-none-match
        X_AMZ_COPY_SOURCE_IF_NONE_MATCH: "x-amz-copy-source-if-none-match",

        /// x-amz-copy-source-if-unmodified-since
        X_AMZ_COPY_SOURCE_IF_UNMODIFIED_SINCE: "x-amz-copy-source-if-unmodified-since",

        /// x-amz-grant-full-control
        X_AMZ_GRANT_FULL_CONTROL: "x-amz-grant-full-control",

        /// x-amz-grant-read
        X_AMZ_GRANT_READ: "x-amz-grant-read",

        /// x-amz-grant-read-acp
        X_AMZ_GRANT_READ_ACP: "x-amz-grant-read-acp",

        /// x-amz-grant-write-acp
        X_AMZ_GRANT_WRITE_ACP: "x-amz-grant-write-acp",

        /// x-amz-metadata-directive
        X_AMZ_METADATA_DIRECTIVE: "x-amz-metadata-directive",

        /// x-amz-tagging-directive
        X_AMZ_TAGGING_DIRECTIVE: "x-amz-tagging-directive",

        /// x-amz-server-side-encryption
        X_AMZ_SERVER_SIDE_ENCRYPTION: "x-amz-server-side-encryption",

        /// x-amz-storage-class
        X_AMZ_STORAGE_CLASS: "x-amz-storage-class",

        /// x-amz-website-redirect-location
        X_AMZ_WEBSITE_REDIRECT_LOCATION: "x-amz-website-redirect-location",

        /// x-amz-server-side-encryption-customer-algorithm
        X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM: "x-amz-server-side-encryption-customer-algorithm",

        /// x-amz-server-side-encryption-customer-key
        X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY: "x-amz-server-side-encryption-customer-key",

        /// x-amz-server-side-encryption-customer-key-MD5
        X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5: "x-amz-server-side-encryption-customer-key-md5",

        /// x-amz-server-side-encryption-aws-kms-key-id
        X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID: "x-amz-server-side-encryption-aws-kms-key-id",

        /// x-amz-server-side-encryption-context
        X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT: "x-amz-server-side-encryption-context",

        /// x-amz-copy-source-server-side-encryption-customer-algorithm
        X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM: "x-amz-copy-source-server-side-encryption-customer-algorithm",

        /// x-amz-copy-source-server-side-encryption-customer-key
        X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY: "x-amz-copy-source-server-side-encryption-customer-key",

        /// x-amz-copy-source-server-side-encryption-customer-key-MD5
        X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5: "x-amz-copy-source-server-side-encryption-customer-key-md5",

        /// x-amz-request-payer
        X_AMZ_REQUEST_PAYER: "x-amz-request-payer",

        /// x-amz-tagging
        X_AMZ_TAGGING: "x-amz-tagging",

        /// x-amz-object-lock-mode
        X_AMZ_OBJECT_LOCK_MODE: "x-amz-object-lock-mode",

        /// x-amz-object-lock-retain-until-date
        X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE: "x-amz-object-lock-retain-until-date",

        /// x-amz-object-lock-legal-hold
        X_AMZ_OBJECT_LOCK_LEGAL_HOLD: "x-amz-object-lock-legal-hold",
    ];
}
