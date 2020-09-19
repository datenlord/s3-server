//! crypto utils

use hmac::{Hmac, Mac, NewMac};
use sha2::{Digest, Sha256};

use crate::utils::Also;

/// verify sha256 checksum string
pub fn is_sha256_checksum(s: &str) -> bool {
    let is_lowercase_hex = |&c: &u8| c.is_ascii_digit() || (b'a'..=b'f').contains(&c);
    s.len() == 64 && s.as_bytes().iter().all(is_lowercase_hex)
}

/// `hex(sha256(data))`
pub fn hex_sha256(data: &[u8]) -> String {
    let src = Sha256::digest(data);

    #[cfg(test)]
    debug_assert!(src.as_slice().len() == 32);

    #[allow(clippy::unreachable)] // a sha256 hash string's length is always 64
    faster_hex::hex_string(src.as_ref()).unwrap_or_else(|_| unreachable!())
}

/// `hmac_sha256(key, data)`
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> impl AsRef<[u8]> {
    #[allow(clippy::unreachable)] // HMAC can take key of any size
    let m = <Hmac<Sha256>>::new_varkey(key).unwrap_or_else(|_| unreachable!());
    m.also(|m| m.update(data.as_ref())).finalize().into_bytes()
}

/// `hex(hmac_sha256(key, data))`
pub fn hex_hmac_sha256(key: &[u8], data: &[u8]) -> String {
    let src = hmac_sha256(key, data);

    #[cfg(test)]
    debug_assert!(src.as_ref().len() == 32);

    #[allow(clippy::unreachable)] // a hmac sha256 hash string's length is always 64
    faster_hex::hex_string(src.as_ref()).unwrap_or_else(|_| unreachable!())
}
