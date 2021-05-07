//! crypto utils

use crate::utils::Also;

use hmac::{Hmac, Mac, NewMac};
use hyper::body::Bytes;
use sha2::{Digest, Sha256};

/// convert bytes to hex string
pub fn to_hex_string(src: impl AsRef<[u8]>) -> String {
    faster_hex::hex_string(src.as_ref()).unwrap_or_else(|e| panic!("{}", e))
}

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

    to_hex_string(src)
}

/// `hex(sha256(chunks))`
pub fn hex_sha256_chunk(chunk_data: &[Bytes]) -> String {
    let src = Sha256::new()
        .also(|h| chunk_data.iter().for_each(|data| h.update(data)))
        .finalize();

    #[cfg(test)]
    debug_assert!(src.as_slice().len() == 32);

    to_hex_string(src)
}

/// `hmac_sha256(key, data)`
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> impl AsRef<[u8]> {
    let m = <Hmac<Sha256>>::new_from_slice(key)
        .unwrap_or_else(|_| panic!("HMAC can take key of any size"));
    m.also(|m| m.update(data.as_ref())).finalize().into_bytes()
}

/// `hex(hmac_sha256(key, data))`
pub fn hex_hmac_sha256(key: &[u8], data: &[u8]) -> String {
    let src = hmac_sha256(key, data);

    #[cfg(test)]
    debug_assert!(src.as_ref().len() == 32);

    to_hex_string(src)
}

/// is base64 encoded
pub fn is_base64_encoded(bytes: &[u8]) -> bool {
    if bytes.len().wrapping_rem(4) != 0 {
        return false;
    }

    // TODO: benchmark which is faster
    // + base64::decode_config_buf
    // + use lookup table, check `=` and length
    let mut buf = Vec::new();
    base64::decode_config_buf(bytes, base64::STANDARD, &mut buf).is_ok()
}
