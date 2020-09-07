#![allow(dead_code)] // TODO: remove this

//! Common Request Headers

use crate::utils::{is_sha256_checksum, Apply};

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
    /// aws4_request
    aws4_request: (),
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
                    aws4_request: (),
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
