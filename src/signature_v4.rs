//! AWS Signature Version 4
//!
//! See <https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html>
//!

use crate::headers::AmzDate;
use crate::utils::{crypto, Also, Apply, OrderedHeaders};

use hyper::Method;

use smallvec::SmallVec;

/// custom uri encode
fn uri_encode(output: &mut String, input: &str, encode_slash: bool) {
    /// hex uppercase table
    const HEX_UPPERCASE_TABLE: [u8; 16] = *b"0123456789ABCDEF";

    let mut buf: SmallVec<[u8; 512]> = SmallVec::with_capacity(input.len());

    for &byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'~' | b'.' => buf.push(byte),
            b'/' => {
                if encode_slash {
                    buf.push(b'%');
                    buf.push(b'2');
                    buf.push(b'F');
                } else {
                    buf.push(byte);
                }
            }
            _ => {
                macro_rules! to_hex {
                    ($n:expr) => {{
                        #[allow(clippy::unreachable)]
                        *HEX_UPPERCASE_TABLE
                            .get(usize::from($n))
                            .unwrap_or_else(|| unreachable!()) // a 4-bits number is always less then 16
                    }};
                }

                buf.push(b'%');
                buf.push(to_hex!(byte.wrapping_shr(4)));
                buf.push(to_hex!(byte & 15));
            }
        }
    }

    #[allow(clippy::unreachable)]
    std::str::from_utf8(buf.as_ref())
        .unwrap_or_else(|_| unreachable!()) // an ascii string is always a utf-8 string
        .apply(|s| output.push_str(s))
}

/// is skipped header
fn is_skipped_header(header: &str) -> bool {
    ["authorization", "content-length", "user-agent"].contains(&header)
}

/// sha256 hash of an empty string
const EMPTY_STRING_SHA256_HASH: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// create canonical request
pub fn create_canonical_request(
    method: &Method,
    uri_path: &str,
    query_strings: &[(String, String)],
    headers: &OrderedHeaders<'_>,
    payload: &[u8],
) -> String {
    String::with_capacity(256)
        .also(|ans| {
            // <HTTPMethod>\n
            ans.push_str(method.as_str());
            ans.push('\n');
        })
        .also(|ans| {
            // <CanonicalURI>\n
            uri_encode(ans, uri_path, false);
            ans.push('\n');
        })
        .also(|ans| {
            // <CanonicalQueryString>\n
            let encoded_query_strings: SmallVec<[(String, String); 16]> = query_strings
                .iter()
                .map(|(n, v)| {
                    let name = String::with_capacity(n.len()).also(|s| uri_encode(s, n, true));
                    let value = String::with_capacity(v.len()).also(|s| uri_encode(s, v, true));
                    (name, value)
                })
                .collect::<SmallVec<[(String, String); 16]>>()
                .also(|qs| qs.sort());

            if let Some((first, remain)) = encoded_query_strings.split_first() {
                {
                    let (name, value) = first;
                    ans.push_str(name);
                    ans.push('=');
                    ans.push_str(value);
                }
                for (name, value) in remain {
                    ans.push('&');
                    ans.push_str(name);
                    ans.push('=');
                    ans.push_str(value);
                }
            }

            ans.push('\n');
        })
        .also(|ans| {
            // <CanonicalHeaders>\n

            // FIXME: check HOST, Content-Type, x-amz-security-token, x-amz-content-sha256

            for &(name, value) in headers.as_ref().iter() {
                if is_skipped_header(name) {
                    continue;
                }
                ans.push_str(name);
                ans.push(':');
                ans.push_str(value.trim());
                ans.push('\n');
            }
            ans.push('\n');
        })
        .also(|ans| {
            // <SignedHeaders>\n
            let mut first_flag = true;
            for &(name, _) in headers.as_ref().iter() {
                if is_skipped_header(name) {
                    continue;
                }
                if first_flag {
                    first_flag = false;
                } else {
                    ans.push(';');
                }
                ans.push_str(name);
            }

            ans.push('\n');
        })
        .also(|ans| {
            // <HashedPayload>
            if payload.is_empty() {
                ans.push_str(EMPTY_STRING_SHA256_HASH);
            } else {
                ans.push_str(&crypto::hex_sha256(payload));
            }
        })
}

/// create string to sign
pub fn create_string_to_sign(canonical_request: &str, amz_date: &AmzDate, region: &str) -> String {
    String::with_capacity(256)
        .also(|ans| {
            // <Algorithm>\n
            ans.push_str("AWS4-HMAC-SHA256\n");
        })
        .also(|ans| {
            // <RequestDateTime>\n
            ans.push_str(&amz_date.to_iso8601());
            ans.push('\n');
        })
        .also(|ans| {
            // <CredentialScope>\n
            ans.push_str(&amz_date.to_date());
            ans.push('/');
            ans.push_str(region); // TODO: use a `Region` type
            ans.push_str("/s3/aws4_request\n");
        })
        .also(|ans| {
            // <HashedCanonicalRequest>
            ans.push_str(&crypto::hex_sha256(canonical_request.as_bytes()))
        })
}

/// calculate signature
pub fn calculate_signature(
    string_to_sign: &str,
    secret_key: &str,
    amz_date: &AmzDate,
    region: &str,
) -> String {
    let secret = <SmallVec<[u8; 128]>>::with_capacity(secret_key.len().saturating_add(4))
        .also(|v| v.extend_from_slice(b"AWS4"))
        .also(|v| v.extend_from_slice(secret_key.as_bytes()));

    let date = amz_date.to_date();

    // DateKey
    let date_key = crypto::hmac_sha256(secret.as_ref(), date.as_ref());

    // DateRegionKey
    let date_region_key = crypto::hmac_sha256(date_key.as_ref(), region.as_ref()); // TODO: use a `Region` type

    // DateRegionServiceKey
    let date_region_service_key = crypto::hmac_sha256(date_region_key.as_ref(), "s3".as_ref());

    // SigningKey
    let signing_key =
        crypto::hmac_sha256(date_region_service_key.as_ref(), "aws4_request".as_ref());

    // Signature
    crypto::hex_hmac_sha256(signing_key.as_ref(), string_to_sign.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_get_object() {
        // let access_key_id = "AKIAIOSFODNN7EXAMPLE";
        let secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let timestamp = "20130524T000000Z";
        // let bucket = "examplebucket";
        let region = "us-east-1";
        let path = "/test.txt";

        let headers = OrderedHeaders::from_slice_unchecked(&[
            ("host", "examplebucket.s3.amazonaws.com"),
            ("range", "bytes=0-9"),
            (
                "x-amz-content-sha256",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            ("x-amz-date", "20130524T000000Z"),
        ]);

        let method = Method::GET;

        let canonical_request = create_canonical_request(&method, path, &[], &headers, &[]);
        assert_eq!(
            canonical_request,
            concat!(
                "GET\n",
                "/test.txt\n",
                "\n",
                "host:examplebucket.s3.amazonaws.com\n",
                "range:bytes=0-9\n",
                "x-amz-content-sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n",
                "x-amz-date:20130524T000000Z\n",
                "\n",
                "host;range;x-amz-content-sha256;x-amz-date\n",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            )
        );

        let date = AmzDate::from_header_str(timestamp).unwrap();
        let string_to_sign = create_string_to_sign(&canonical_request, &date, region);
        assert_eq!(
            string_to_sign,
            concat!(
                "AWS4-HMAC-SHA256\n",
                "20130524T000000Z\n",
                "20130524/us-east-1/s3/aws4_request\n",
                "7344ae5b7ee6c3e7e6b0fe0640412a37625d1fbfff95c48bbb2dc43964946972",
            )
        );

        let signature = calculate_signature(&string_to_sign, secret_access_key, &date, region);
        assert_eq!(
            signature,
            "f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }

    #[test]
    fn example_put_object() {
        // let access_key_id = "AKIAIOSFODNN7EXAMPLE";
        let secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let timestamp = "20130524T000000Z";
        // let bucket = "examplebucket";
        let region = "us-east-1";
        let path = "/test$file.text";

        let headers = OrderedHeaders::from_slice_unchecked(&[
            ("date", "Fri, 24 May 2013 00:00:00 GMT"),
            ("host", "examplebucket.s3.amazonaws.com"),
            (
                "x-amz-content-sha256",
                "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072",
            ),
            ("x-amz-date", "20130524T000000Z"),
            ("x-amz-storage-class", "REDUCED_REDUNDANCY"),
        ]);

        let method = Method::PUT;
        let payload = "Welcome to Amazon S3.";

        let canonical_request =
            create_canonical_request(&method, path, &[], &headers, payload.as_bytes());

        assert_eq!(
            canonical_request,
            concat!(
                "PUT\n",
                "/test%24file.text\n",
                "\n",
                "date:Fri, 24 May 2013 00:00:00 GMT\n",
                "host:examplebucket.s3.amazonaws.com\n",
                "x-amz-content-sha256:44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072\n",
                "x-amz-date:20130524T000000Z\n",
                "x-amz-storage-class:REDUCED_REDUNDANCY\n",
                "\n",
                "date;host;x-amz-content-sha256;x-amz-date;x-amz-storage-class\n",
                "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072",
            )
        );

        let date = AmzDate::from_header_str(timestamp).unwrap();
        let string_to_sign = create_string_to_sign(&canonical_request, &date, region);
        assert_eq!(
            string_to_sign,
            concat!(
                "AWS4-HMAC-SHA256\n",
                "20130524T000000Z\n",
                "20130524/us-east-1/s3/aws4_request\n",
                "9e0e90d9c76de8fa5b200d8c849cd5b8dc7a3be3951ddb7f6a76b4158342019d",
            )
        );

        let signature = calculate_signature(&string_to_sign, secret_access_key, &date, region);
        assert_eq!(
            signature,
            "98ad721746da40c64f1a55b78f14c238d841ea1380cd77a1b5971af0ece108bd"
        );
    }

    #[test]
    fn example_get_bucket_lifecycle_configuration() {
        // let access_key_id = "AKIAIOSFODNN7EXAMPLE";
        let secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let timestamp = "20130524T000000Z";
        // let bucket = "examplebucket";
        let region = "us-east-1";
        let path = "/";

        let headers = OrderedHeaders::from_slice_unchecked(&[
            ("host", "examplebucket.s3.amazonaws.com"),
            (
                "x-amz-content-sha256",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            ("x-amz-date", "20130524T000000Z"),
        ]);

        let query_strings = &[("lifecycle".into(), "".into())];

        let method = Method::GET;

        let canonical_request =
            create_canonical_request(&method, path, query_strings, &headers, &[]);
        assert_eq!(
            canonical_request,
            concat!(
                "GET\n",
                "/\n",
                "lifecycle=\n",
                "host:examplebucket.s3.amazonaws.com\n",
                "x-amz-content-sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n",
                "x-amz-date:20130524T000000Z\n",
                "\n",
                "host;x-amz-content-sha256;x-amz-date\n",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            )
        );

        let date = AmzDate::from_header_str(timestamp).unwrap();
        let string_to_sign = create_string_to_sign(&canonical_request, &date, region);
        assert_eq!(
            string_to_sign,
            concat!(
                "AWS4-HMAC-SHA256\n",
                "20130524T000000Z\n",
                "20130524/us-east-1/s3/aws4_request\n",
                "9766c798316ff2757b517bc739a67f6213b4ab36dd5da2f94eaebf79c77395ca",
            )
        );

        let signature = calculate_signature(&string_to_sign, secret_access_key, &date, region);
        assert_eq!(
            signature,
            "fea454ca298b7da1c68078a5d1bdbfbbe0d65c699e0f91ac7a200a0136783543"
        );
    }

    #[test]
    fn example_list_objects() {
        // let access_key_id = "AKIAIOSFODNN7EXAMPLE";
        let secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let timestamp = "20130524T000000Z";
        // let bucket = "examplebucket";
        let region = "us-east-1";
        let path = "/";

        let headers = OrderedHeaders::from_slice_unchecked(&[
            ("host", "examplebucket.s3.amazonaws.com"),
            (
                "x-amz-content-sha256",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            ("x-amz-date", "20130524T000000Z"),
        ]);

        let query_strings = &[
            ("max-keys".into(), "2".into()),
            ("prefix".into(), "J".into()),
        ];

        let method = Method::GET;

        let canonical_request =
            create_canonical_request(&method, path, query_strings, &headers, &[]);

        assert_eq!(
            canonical_request,
            concat!(
                "GET\n",
                "/\n",
                "max-keys=2&prefix=J\n",
                "host:examplebucket.s3.amazonaws.com\n",
                "x-amz-content-sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n",
                "x-amz-date:20130524T000000Z\n",
                "\n",
                "host;x-amz-content-sha256;x-amz-date\n",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            )
        );

        let date = AmzDate::from_header_str(timestamp).unwrap();
        let string_to_sign = create_string_to_sign(&canonical_request, &date, region);
        assert_eq!(
            string_to_sign,
            concat!(
                "AWS4-HMAC-SHA256\n",
                "20130524T000000Z\n",
                "20130524/us-east-1/s3/aws4_request\n",
                "df57d21db20da04d7fa30298dd4488ba3a2b47ca3a489c74750e0f1e7df1b9b7",
            )
        );

        let signature = calculate_signature(&string_to_sign, secret_access_key, &date, region);
        assert_eq!(
            signature,
            "34b48302e7b5fa45bde8084f4b7868a86f0a534bc59db6670ed5711ef69dc6f7"
        );
    }
}
