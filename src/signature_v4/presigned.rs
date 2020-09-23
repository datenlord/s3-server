//! presigned request

use crate::headers::{AmzDate, CredentialV4};
use crate::utils::{crypto, Apply, OrderedQs};

/// query strings of a presigned url
#[derive(Debug)]
pub struct PresignedQs<'a> {
    /// X-Amz-Algorithm
    x_amz_algorithm: &'a str,
    /// X-Amz-Credential
    x_amz_credential: &'a str,
    /// X-Amz-Date
    x_amz_date: &'a str,
    /// X-Amz-Expires
    x_amz_expires: &'a str,
    /// X-Amz-SignedHeaders
    x_amz_signed_headers: &'a str,
    /// X-Amz-Signature
    x_amz_signature: &'a str,
}

/// presigned url information
pub struct PresignedUrl<'a> {
    /// algorithm
    pub algorithm: &'a str,
    /// credential
    pub credential: CredentialV4<'a>,
    /// amz date
    pub amz_date: AmzDate,
    /// expires
    pub expires: u32,
    /// signed headers
    pub signed_headers: Vec<&'a str>,
    /// signature
    pub signature: &'a str,
}

/// `ParsePresignedUrlError`
#[allow(missing_copy_implementations)]
#[derive(Debug, thiserror::Error)]
#[error("ParsePresignedUrlError")]
pub struct ParsePresignedUrlError {
    /// priv place holder
    _priv: (),
}

impl<'a> PresignedUrl<'a> {
    /// parse `PresignedUrl` from query
    pub fn from_query(qs: &'a OrderedQs) -> Result<Self, ParsePresignedUrlError> {
        let get_info = || -> Option<PresignedQs<'a>> {
            PresignedQs {
                x_amz_algorithm: qs.get("X-Amz-Algorithm")?,
                x_amz_credential: qs.get("X-Amz-Credential")?,
                x_amz_date: qs.get("X-Amz-Date")?,
                x_amz_expires: qs.get("X-Amz-Expires")?,
                x_amz_signed_headers: qs.get("X-Amz-SignedHeaders")?,
                x_amz_signature: qs.get("X-Amz-Signature")?,
            }
            .apply(Some)
        };
        let info = get_info().ok_or_else(|| ParsePresignedUrlError { _priv: () })?;

        let algorithm = info.x_amz_algorithm;

        let credential = match CredentialV4::parse_by_nom(info.x_amz_credential) {
            Ok(("", c)) => c,
            Ok(_) | Err(_) => return Err(ParsePresignedUrlError { _priv: () }),
        };

        let amz_date = AmzDate::from_header_str(info.x_amz_date)
            .map_err(|_| ParsePresignedUrlError { _priv: () })?;

        let expires: u32 = info
            .x_amz_expires
            .parse()
            .map_err(|_| ParsePresignedUrlError { _priv: () })?;

        if !info.x_amz_signed_headers.is_ascii() {
            return Err(ParsePresignedUrlError { _priv: () });
        }
        let signed_headers = info.x_amz_signed_headers.split(';').collect::<Vec<&str>>();

        if !crypto::is_sha256_checksum(info.x_amz_signature) {
            return Err(ParsePresignedUrlError { _priv: () });
        }
        let signature = info.x_amz_signature;

        Self {
            algorithm,
            credential,
            amz_date,
            expires,
            signed_headers,
            signature,
        }
        .apply(Ok)
    }
}
