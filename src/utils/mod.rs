//! utils

mod also;
mod apply;
mod byte_stream;
mod request;
mod response;
mod xml;

use hyper::Body;
use serde::de::DeserializeOwned;

use crate::BoxStdError;

pub use self::also::Also;
pub use self::apply::Apply;
pub use self::byte_stream::ByteStream;
pub use self::request::RequestExt;
pub use self::response::ResponseExt;
pub use self::xml::XmlWriterExt;

pub mod time;
pub mod crypto;

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
