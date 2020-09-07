//! utils

mod apply;
mod byte_stream;
mod request;
mod response;
mod xml;

pub use self::apply::Apply;
pub use self::byte_stream::ByteStream;
pub use self::request::RequestExt;
pub use self::response::ResponseExt;
pub use self::xml::XmlWriterExt;

pub mod time;

#[allow(unused_macros)]
macro_rules! cfg_rt_tokio{
    {$($item:item)*}=>{
        $(
            #[cfg(feature = "rt-tokio")]
            $item
        )*
    }
}

/// verify sha256 checksum string
pub fn is_sha256_checksum(s: &str) -> bool {
    let is_lowercase_hex = |&c: &u8| c.is_ascii_digit() || (b'a'..=b'f').contains(&c);
    s.len() == 64 && s.as_bytes().iter().all(is_lowercase_hex)
}
