//! utils

mod byte_stream;

pub(crate) use self::byte_stream::ByteStream;


#[allow(unused_macros)]
macro_rules! cfg_rt_tokio{
    {$($item:item)*}=>{
        $(
            #[cfg(feature = "rt-tokio")]
            $item
        )*
    }
}

use hyper::{
    header::{self, HeaderValue, InvalidHeaderValue},
    Body, StatusCode,
};
use mime::Mime;
use std::convert::TryFrom;
use std::io;
use xml::writer::{events::XmlEvent, EventWriter};

/// Request type
pub(super) type Request = hyper::Request<Body>;

/// Response type
pub(super) type Response = hyper::Response<Body>;

/// `Box<dyn std::error::Error + Send + Sync + 'static>`
pub(super) type BoxStdError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// helper function for writing xml
pub(super) fn xml_write_string_element<W: io::Write>(
    w: &mut EventWriter<W>,
    name: &str,
    data: &str,
) -> xml::writer::Result<()> {
    w.write(XmlEvent::start_element(name))?;
    w.write(XmlEvent::characters(data))?;
    w.write(XmlEvent::end_element())?;
    Ok(())
}

/// helper function for setting Mime
pub(super) fn set_mime(res: &mut Response, mime: &Mime) -> Result<(), InvalidHeaderValue> {
    let val = HeaderValue::try_from(mime.as_ref())?;
    let _ = res.headers_mut().insert(header::CONTENT_TYPE, val);
    Ok(())
}

/// helper function for creating response
pub(super) fn create_response(body: impl Into<Body>, status: Option<StatusCode>) -> Response {
    let mut res = Response::new(body.into());
    if let Some(status) = status {
        *res.status_mut() = status
    }
    res
}
