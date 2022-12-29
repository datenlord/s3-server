//! Authorization
//!
//! See [sigv4-auth-using-authorization-header](https://docs.aws.amazon.com/AmazonS3/latest/API/sigv4-auth-using-authorization-header.html)
//!

use crate::utils::Apply;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Authorization
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::exhaustive_structs)]
pub struct AuthorizationV4<'a> {
    /// The algorithm that was used to calculate the signature.
    pub algorithm: &'a str,

    /// Access key ID and the scope information, which includes the date, Region, and service that were used to calculate the signature.
    pub credential: CredentialV4<'a>,

    /// A semicolon-separated list of request headers that you used to compute `Signature`.
    pub signed_headers: Vec<&'a str>,

    /// The 256-bit signature expressed as 64 lowercase hexadecimal characters.
    pub signature: &'a str,
}

/// Access key ID and the scope information, which includes the date, Region, and service that were used to calculate the signature.
///
/// This string has the following form:
/// `<your-access-key-id>/<date>/<aws-region>/<aws-service>/aws4_request`
///
/// See [sigv4-auth-using-authorization-header](https://docs.aws.amazon.com/AmazonS3/latest/API/sigv4-auth-using-authorization-header.html)
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::exhaustive_structs)]
pub struct CredentialV4<'a> {
    /// access key id
    pub access_key_id: &'a str,
    /// <date> value is specified using YYYYMMDD format.
    pub date: &'a str,
    /// region
    pub aws_region: &'a str,
    /// <aws-service> value is `s3` when sending request to Amazon S3.
    pub aws_service: &'a str,
}

/// helper macro for parser
macro_rules! parse_and_bind {
    (mut $input:expr => $f:expr => $id:pat ) => {
        let $id = $f($input)?.apply(|(__input, output)| {
            $input = __input;
            output
        });
    };
    ($input:expr => $f:expr => $id:pat ) => {
        let $id = $f($input)?.apply(|(_, output)| output);
    };
}

impl<'a> CredentialV4<'a> {
    /// parse by nom
    pub(crate) fn parse_by_nom(mut input: &'a str) -> nom::IResult<&'a str, Self> {
        use nom::{
            bytes::complete::{tag, take, take_till, take_till1},
            sequence::terminated,
        };

        let mut slash_tail1 = terminated(take_till1(|c| c == '/'), take(1_usize));
        let mut slash_tail0 = terminated(take_till(|c| c == '/'), take(1_usize));

        parse_and_bind!(mut input => slash_tail1 => access_key_id);
        parse_and_bind!(mut input => slash_tail1 => date);
        parse_and_bind!(date => Self::verify_date => _);
        parse_and_bind!(mut input => slash_tail0 => aws_region);
        parse_and_bind!(mut input => slash_tail1 => aws_service);
        parse_and_bind!(mut input => tag("aws4_request") => _);

        CredentialV4 {
            access_key_id,
            date,
            aws_region,
            aws_service,
        }
        .apply(|c| Ok((input, c)))
    }

    /// verify date: YYYYMMDD
    fn verify_date(input: &str) -> nom::IResult<&str, (&str, &str, &str)> {
        use chrono::{TimeZone, Utc};
        use nom::{
            bytes::complete::take,
            combinator::{all_consuming, verify},
            sequence::tuple,
        };

        verify(
            all_consuming(tuple((take(4_usize), take(2_usize), take(2_usize)))),
            |&(y, m, d): &(&str, &str, &str)| {
                /// helper macro
                macro_rules! parse_num {
                    ($x:expr) => {{
                        match $x.parse() {
                            Ok(x) => x,
                            Err(_) => return false,
                        }
                    }};
                }
                matches!(
                    Utc.with_ymd_and_hms(parse_num!(y), parse_num!(m), parse_num!(d), 0, 0, 0),
                    chrono::LocalResult::Single(_)
                )
            },
        )(input)
    }
}

/// `ParseAuthorizationError`
#[allow(missing_copy_implementations)] // Why? See `crate::path::ParseS3PathError`.
#[derive(Debug, thiserror::Error)]
#[error("ParseAuthorizationError")]
pub struct ParseAuthorizationError {
    /// priv place holder
    _priv: (),
}

impl<'a> AuthorizationV4<'a> {
    /// parse `AuthorizationV4` from `Authorization` header
    /// # Errors
    /// Returns an `Err` if the header is invalid
    pub fn from_header_str(auth: &'a str) -> Result<Self, ParseAuthorizationError> {
        /// nom parser
        fn parse(mut input: &str) -> nom::IResult<&str, AuthorizationV4<'_>> {
            use nom::{
                bytes::complete::{tag, take, take_till, take_till1},
                character::complete::{multispace0, multispace1},
                combinator::all_consuming,
                sequence::tuple,
            };

            let space_till1 = take_till1(|c: char| c.is_ascii_whitespace());
            let space_till0 = take_till(|c: char| c.is_ascii_whitespace());

            parse_and_bind!(mut input => space_till1 => algorithm);
            parse_and_bind!(mut input => multispace1 => _);
            parse_and_bind!(mut input => tag("Credential=") => _);
            parse_and_bind!(mut input => CredentialV4::parse_by_nom => credential);
            parse_and_bind!(mut input => tag(",") => _);
            parse_and_bind!(mut input => multispace0 => _);
            parse_and_bind!(mut input => tag("SignedHeaders=") => _);

            let mut headers: SmallVec<[&str; 16]> = SmallVec::new();
            loop {
                let mut expect_header =
                    tuple((take_till1(|c| c == ';' || c == ','), take(1_usize)));
                parse_and_bind!(mut input => expect_header => (header, sep));
                headers.push(header);
                if sep == "," {
                    break;
                }
            }

            parse_and_bind!(mut input => multispace0 => _);
            parse_and_bind!(mut input => tag("Signature=") => _);
            parse_and_bind!(mut input => space_till0 => signature);
            parse_and_bind!(mut input => all_consuming(multispace0) => _);

            let ans = AuthorizationV4 {
                algorithm,
                credential,
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

    #[test]
    fn regression() {
        let auth = concat!(
            "AWS4-HMAC-SHA256 ",
            "Credential=AKIAIOSFODNN7EXAMPLE/20200921/us-east-1/s3/aws4_request,",
            "SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-decoded-content-length,",
            "Signature=7a7f7778618cadc05f112b44cca218e001a0a020c5c512d8aa2bca2afb713fad",
        );

        let ans = AuthorizationV4::from_header_str(auth).unwrap();

        assert_eq!(ans.algorithm, "AWS4-HMAC-SHA256");
        assert_eq!(ans.credential.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(ans.credential.date, "20200921");
        assert_eq!(ans.credential.aws_region, "us-east-1");
        assert_eq!(ans.credential.aws_service, "s3");
        assert_eq!(
            ans.signed_headers,
            &[
                "host",
                "x-amz-content-sha256",
                "x-amz-date",
                "x-amz-decoded-content-length"
            ]
        );
        assert_eq!(
            ans.signature,
            "7a7f7778618cadc05f112b44cca218e001a0a020c5c512d8aa2bca2afb713fad"
        );
    }
}
