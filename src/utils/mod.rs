//! utils

mod apply;
mod byte_stream;
mod request;
mod response;
mod xml;

use hyper::Body;
use serde::de::DeserializeOwned;

use crate::BoxStdError;

pub use self::apply::Apply;
pub use self::byte_stream::ByteStream;
pub use self::request::RequestExt;
pub use self::response::ResponseExt;
pub use self::xml::XmlWriterExt;

pub mod time;

/// verify sha256 checksum string
pub fn is_sha256_checksum(s: &str) -> bool {
    let is_lowercase_hex = |&c: &u8| c.is_ascii_digit() || (b'a'..=b'f').contains(&c);
    s.len() == 64 && s.as_bytes().iter().all(is_lowercase_hex)
}

macro_rules! assign_opt{
    (from $src:ident to $dst:ident fields [$($field: tt,)+])=>{$(
        if $src.$field.is_some(){
            $dst.$field = $src.$field;
        }
    )+};

    (from $req:ident to $input:ident headers [$($name:expr => $field:ident,)+])=>{{
        let _ = $req
        $(
            .assign_opt_header($name, &mut $input.$field)?
        )+
        ;
    }};
}

/// deserialize xml body
pub async fn deserialize_xml_body<T: DeserializeOwned>(body: Body) -> Result<T, BoxStdError> {
    let bytes = hyper::body::to_bytes(body).await?;
    let ans: T = quick_xml::de::from_reader(&*bytes)?;
    Ok(ans)
}

macro_rules! static_regex {
    ($re: literal) => {{
        use once_cell::sync::Lazy;
        use regex::Regex;

        // compile-time verified regex
        const RE: &'static str = const_str::verified_regex!($re);

        static PATTERN: Lazy<Regex> =
            Lazy::new(|| Regex::new(RE).unwrap_or_else(|e| panic!("static regex error: {}", e)));

        &*PATTERN
    }};
}
